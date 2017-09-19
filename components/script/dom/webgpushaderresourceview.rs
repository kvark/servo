/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::ShaderResourceViewInfo;
use dom::bindings::codegen::Bindings::WebGpuShaderResourceViewBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::reflect_dom_object;
use dom::globalscope::GlobalScope;
use dom::webgpudescriptor::WebGpuDescriptor;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuShaderResourceView {
    descriptor: WebGpuDescriptor,
}

impl WebGpuShaderResourceView {
    pub fn new(global: &GlobalScope, info: ShaderResourceViewInfo) -> Root<Self> {
        let obj = box WebGpuShaderResourceView {
            descriptor: WebGpuDescriptor::new_inherited(info.id),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}
