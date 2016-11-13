/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;

#[dom_struct]
pub struct WebMetalBuffer {
    reflector: Reflector,
}

impl WebMetalBuffer {
    pub fn new(global: &GlobalScope) -> Root<WebMetalBuffer> {
        let object = box WebMetalBuffer {
            reflector: Reflector::new(),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

//impl binding::WebMetalBufferMethods for WebMetalBuffer {
//}
