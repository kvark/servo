/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebMetalCommandBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use webmetal;

#[dom_struct]
pub struct WebMetalCommandBuffer {
    reflector: Reflector,
    #[ignore_heap_size_of = "Defined in webmetal"]
    com_buf: webmetal::CommandBuffer,
}

impl WebMetalCommandBuffer {
    pub fn new(global: &GlobalScope, com: webmetal::CommandBuffer)
               -> Root<WebMetalCommandBuffer> {
        let object = box WebMetalCommandBuffer {
            reflector: Reflector::new(),
            com_buf: com,
        };
        reflect_dom_object(object, global, binding::Wrap)
    }
}

//impl binding::WebMetalCommandBufferMethods for WebMetalCommandBuffer {
//}
