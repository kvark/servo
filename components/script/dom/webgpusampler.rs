/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::SamplerInfo;
use dom::bindings::codegen::Bindings::WebGpuSamplerBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::reflect_dom_object;
use dom::globalscope::GlobalScope;
use dom::webgpudescriptor::WebGpuDescriptor;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuSampler {
    descriptor: WebGpuDescriptor,
}

impl WebGpuSampler {
    pub fn new(global: &GlobalScope, info: SamplerInfo) -> Root<Self> {
        let obj = box WebGpuSampler {
            descriptor: WebGpuDescriptor::new_inherited(info.id),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}
