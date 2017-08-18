/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::WebGpuCommandQueueBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuCommandQueue {
    reflector_: Reflector,
}

impl WebGpuCommandQueue {
    pub fn new(global: &GlobalScope) -> Root<Self> {
        let obj = box WebGpuCommandQueue {
            reflector_: Reflector::new(),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

//impl binding::WebGpuCommandQueueMethods for WebGpuDevice {
//}
