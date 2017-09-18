/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{ShaderResourceViewId, ShaderResourceViewInfo};
use dom::bindings::codegen::Bindings::WebGpuShaderResourceViewBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuShaderResourceView {
    reflector_: Reflector,
    id: ShaderResourceViewId,
}

impl WebGpuShaderResourceView {
    pub fn new(global: &GlobalScope, info: ShaderResourceViewInfo) -> Root<Self> {
        let obj = box WebGpuShaderResourceView {
            reflector_: Reflector::new(),
            id: info.id,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn get_id(&self) -> ShaderResourceViewId {
        self.id
    }
}
