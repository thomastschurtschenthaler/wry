use icrate::{
  AppKit::{
    NSAlternateKeyMask, NSCommandKeyMask, NSControlKeyMask, NSEvent, NSEventTypeOtherMouseDown,
    NSEventTypeOtherMouseUp, NSShiftKeyMask, NSView,
  },
  Foundation::NSString,
  WebKit::WKWebView,
};
use objc2::{declare::ClassBuilder, runtime::Sel};

pub unsafe fn add_synthetic_mouse_events_methods(decl: &mut ClassBuilder) {
  decl.add_method(
    objc2::sel!(otherMouseDown:),
    other_mouse_down as extern "C" fn(_, _, _),
  );
  decl.add_method(
    objc2::sel!(otherMouseUp:),
    other_mouse_up as extern "C" fn(_, _, _),
  );
}

extern "C" fn other_mouse_down(this: &WKWebView, _sel: Sel, event: &NSEvent) {
  unsafe {
    if event.r#type() == NSEventTypeOtherMouseDown {
      let button_number = event.buttonNumber();
      match button_number {
        // back button
        3 => {
          let js = create_js_mouse_event(this, event, true, true);
          this.evaluateJavaScript_completionHandler(&NSString::from_str(&js), None);
          return;
        }
        // forward button
        4 => {
          let js = create_js_mouse_event(this, event, true, false);
          this.evaluateJavaScript_completionHandler(&NSString::from_str(&js), None);
          return;
        }
        _ => {}
      }
    }

    this.mouseDown(event);
  }
}
extern "C" fn other_mouse_up(this: &WKWebView, _sel: Sel, event: &NSEvent) {
  unsafe {
    if event.r#type() == NSEventTypeOtherMouseUp {
      let button_number = event.buttonNumber();
      match button_number {
        // back button
        3 => {
          let js = create_js_mouse_event(this, event, false, true);
          this.evaluateJavaScript_completionHandler(&NSString::from_str(&js), None);
          return;
        }
        // forward button
        4 => {
          let js = create_js_mouse_event(this, event, false, false);
          this.evaluateJavaScript_completionHandler(&NSString::from_str(&js), None);
          return;
        }
        _ => {}
      }
    }

    this.mouseUp(event);
  }
}

unsafe fn create_js_mouse_event(
  view: &NSView,
  event: &NSEvent,
  down: bool,
  back_button: bool,
) -> String {
  let event_name = if down { "mousedown" } else { "mouseup" };
  // js equivalent https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/button
  let button = if back_button { 3 } else { 4 };
  let mods_flags = event.modifierFlags();
  let window_point = event.locationInWindow();
  let view_point = view.convertPoint_fromView(window_point, None);
  let x = view_point.x as u32;
  let y = view_point.y as u32;
  // js equivalent https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons
  let buttons = NSEvent::pressedMouseButtons();

  format!(
    r#"(() => {{
        const el = document.elementFromPoint({x},{y});
        const ev = new MouseEvent('{event_name}', {{
          view: window,
          button: {button},
          buttons: {buttons},
          x: {x},
          y: {y},
          bubbles: true,
          detail: {detail},
          cancelBubble: false,
          cancelable: true,
          clientX: {x},
          clientY: {y},
          composed: true,
          layerX: {x},
          layerY: {y},
          pageX: {x},
          pageY: {y},
          screenX: window.screenX + {x},
          screenY: window.screenY + {y},
          ctrlKey: {ctrl_key},
          metaKey: {meta_key},
          shiftKey: {shift_key},
          altKey: {alt_key},
        }});
        el.dispatchEvent(ev)
        if (!ev.defaultPrevented && "{event_name}" === "mouseup") {{
          if (ev.button === 3) {{
            window.history.back();
          }}
          if (ev.button === 4) {{
            window.history.forward();
          }}
        }}
      }})()"#,
    event_name = event_name,
    x = x,
    y = y,
    detail = event.clickCount(),
    ctrl_key = mods_flags & NSControlKeyMask == NSControlKeyMask,
    alt_key = mods_flags & NSAlternateKeyMask == NSAlternateKeyMask,
    shift_key = mods_flags & NSShiftKeyMask == NSShiftKeyMask,
    meta_key = mods_flags & NSCommandKeyMask == NSCommandKeyMask,
    button = button,
    buttons = buttons,
  )
}
