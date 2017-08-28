/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{SubmitInfo};
use dom::bindings::codegen::Bindings::WebGpuSubmitBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuSubmit {
    reflector_: Reflector,
    info: SubmitInfo,
}

impl WebGpuSubmit {
    pub fn new(
        global: &GlobalScope,
        info: SubmitInfo,
    ) -> Root<Self> {
        let obj = box WebGpuSubmit {
            reflector_: Reflector::new(),
            info,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn to_info(&self) -> SubmitInfo {
        self.info.clone()
    }
}

impl binding::WebGpuSubmitMethods for WebGpuSubmit {
    fn Dummy(&self) {}
}
