/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;

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

//impl binding::WebMetalDeviceMethods for WebMetalDevice {
//}
