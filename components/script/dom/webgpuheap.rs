/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::HeapId;
use dom::bindings::codegen::Bindings::WebGpuHeapBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuHeap {
    reflector_: Reflector,
    id: HeapId,
}

impl WebGpuHeap {
    #[allow(unrooted_must_root)]
    pub fn _new(global: &GlobalScope, id: HeapId) -> Root<Self> {
        let obj = box WebGpuHeap {
            reflector_: Reflector::new(),
            id,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}
