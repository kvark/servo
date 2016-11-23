/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalbuffer::WebMetalBuffer;
use js::jsapi::{JSContext, JSObject};

#[dom_struct]
pub struct WebMetalDevice {
    reflector: Reflector,
}

impl WebMetalDevice {
    pub fn new(global: &GlobalScope) -> Root<WebMetalDevice> {
        let object = box WebMetalDevice {
            reflector: Reflector::new(),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalDeviceMethods for WebMetalDevice {
    #[allow(unsafe_code)]
    unsafe fn MakeBuffer(&self, _cx: *mut JSContext, _size: u32, _data: *mut JSObject) -> Root<WebMetalBuffer> {
        WebMetalBuffer::new(&self.global())
    }
}
