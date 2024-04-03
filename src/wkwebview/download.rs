use std::{path::PathBuf, ptr::null_mut, rc::Rc};

use block2::Block;
use icrate::{
  Foundation::{NSError, NSString, NSURL},
  WebKit::{WKDownload, WKDownloadDelegate, WKWebView},
};
use objc2::{
  declare::ClassBuilder,
  rc::Id,
  runtime::{AnyObject, NSObject, ProtocolObject, Sel},
};
use std::ffi::c_void;

pub(crate) unsafe fn set_download_delegate(
  webview: &mut NSObject,
  download_delegate: &ProtocolObject<dyn WKDownloadDelegate>,
) {
  let ivar = webview
    .class()
    .instance_variable("DownloadDelegate")
    .unwrap();
  let ivar_delegate = ivar.load_mut(webview);

  *ivar_delegate = download_delegate;
  // (*webview).set_ivar(
  //   "DownloadDelegate",
  //   download_delegate as *mut _ as *mut c_void,
  // );
}

unsafe fn get_download_delegate(this: &WKWebView) -> &ProtocolObject<dyn WKDownloadDelegate> {
  let ivar = this.class().instance_variable("DownloadDelegate").unwrap();
  let value = ivar.load::<&ProtocolObject<dyn WKDownloadDelegate>>(this);
  value

  // let delegate: *mut c_void = *this.get_ivar("DownloadDelegate");
  // delegate as *mut NSObject
}

// Download action handler
extern "C" fn navigation_download_action(
  this: &WKWebView,
  _: Sel,
  _: &AnyObject,
  _: &AnyObject,
  download: &WKDownload,
) {
  unsafe {
    let delegate = get_download_delegate(&this);
    download.setDelegate(Some(delegate));
    // let _: () = msg_send![download, setDelegate: delegate];
  }
}

// Download response handler
extern "C" fn navigation_download_response(
  this: &WKWebView,
  _: Sel,
  _: &AnyObject,
  _: &AnyObject,
  download: &WKDownload,
) {
  unsafe {
    let delegate = get_download_delegate(this);
    download.setDelegate(Some(delegate));
    // let _: () = msg_send![download, setDelegate: delegate];
  }
}

pub(crate) unsafe fn add_download_methods(decl: &mut ClassBuilder) {
  decl.add_ivar::<*mut c_void>("DownloadDelegate");

  decl.add_method(
    objc2::sel!(webView:navigationAction:didBecomeDownload:),
    navigation_download_action as extern "C" fn(_, _, _, _, _),
  );

  decl.add_method(
    objc2::sel!(webView:navigationResponse:didBecomeDownload:),
    navigation_download_response as extern "C" fn(_, _, _, _, _),
  );
}

pub extern "C" fn download_policy(
  this: &NSObject,
  _: Sel,
  download: &WKDownload,
  _response: &AnyObject,
  suggested_path: &NSString,
  completion_handler: &Block<(*const NSURL,), ()>,
) {
  unsafe {
    let request = download.originalRequest().unwrap();
    // objc2::msg_send![download, originalRequest];
    let url = request.URL().unwrap().absoluteString().unwrap();
    // let url = NSString(url);
    // let path = NSString(suggested_path);
    let mut path = PathBuf::from(suggested_path.to_string());
    // let handler = completion_handler as *mut block::Block<(id,), c_void>;

    let function = (*this).get_ivar::<*mut c_void>("started");
    if !function.is_null() {
      let function = &mut *(*function as *mut Box<dyn for<'s> FnMut(String, &mut PathBuf) -> bool>);
      match (function)(url.to_string().to_string(), &mut path) {
        true => {
          let path = NSString::from_str(&path.display().to_string());
          let ns_url = NSURL::fileURLWithPath_isDirectory(&path, false);
          // let nsurl: id = msg_send![class!(NSURL), fileURLWithPath:  isDirectory: false];
          (*completion_handler).call((Id::as_ptr(&ns_url),))
        }
        false => (*completion_handler).call((null_mut(),)),
      };
    } else {
      #[cfg(feature = "tracing")]
      tracing::warn!("WebView instance is dropped! This navigation handler shouldn't be called.");
      (*completion_handler).call((null_mut(),));
    }
  }
}

pub extern "C" fn download_did_finish(this: &NSObject, _: Sel, download: &WKDownload) {
  unsafe {
    let function = this.get_ivar::<*mut c_void>("completed");
    let original_request = download.originalRequest().unwrap();
    let url = original_request.URL().unwrap().absoluteString().unwrap();
    // let original_request: id = msg_send![download, originalRequest];
    // let url: id = msg_send![original_request, URL];
    // let url: id = msg_send![url, absoluteString];
    // let url = NSString(url).to_str().to_string();

    if !function.is_null() {
      let function = &mut *(*function as *mut Rc<dyn for<'s> Fn(String, Option<PathBuf>, bool)>);
      function(url.to_string(), None, true);
    }
  }
}

pub extern "C" fn download_did_fail(
  this: &NSObject,
  _: Sel,
  download: &WKDownload,
  error: &NSError,
  _resume_data: &AnyObject,
) {
  unsafe {
    #[cfg(debug_assertions)]
    {
      let description = error.localizedDescription().to_string();
      eprintln!("Download failed with error: {}", description);
    }

    let original_request = download.originalRequest().unwrap();
    let url = original_request.URL().unwrap().absoluteString().unwrap();
    // let original_request: id = msg_send![download, originalRequest];
    // let url: id = msg_send![original_request, URL];
    // let url: id = msg_send![url, absoluteString];
    // let url = NSString(url).to_str().to_string();

    let function = this.get_ivar::<*mut c_void>("completed");
    if !function.is_null() {
      let function = &mut *(*function as *mut Rc<dyn for<'s> Fn(String, Option<PathBuf>, bool)>);
      function(url.to_string(), None, false);
    }
  }
}
