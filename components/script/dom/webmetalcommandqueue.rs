/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalCommandQueueBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webmetalcommandbuffer::WebMetalCommandBuffer;

#[dom_struct]
pub struct WebMetalCommandQueue {
    reflector: Reflector,
}

impl WebMetalCommandQueue {
    pub fn new(global: &GlobalScope)
               -> Root<WebMetalCommandQueue> {
        let object = box WebMetalCommandQueue {
            reflector: Reflector::new(),
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

impl binding::WebMetalCommandQueueMethods for WebMetalCommandQueue {
    fn MakeCommandBuffer(&self) -> Root<WebMetalCommandBuffer> {
        WebMetalCommandBuffer::new(&self.global())
    }
}
