/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::AdapterInfo;
use dom::bindings::codegen::Bindings::WebGpuAdapterBinding as binding;
use dom::bindings::js::{Root};
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::window::Window;
use dom_struct::dom_struct;
//use webgpu::gpu;


#[dom_struct]
pub struct WebGpuAdapter {
    reflector_: Reflector,
}

impl WebGpuAdapter {
    pub fn new(window: &Window, _info: AdapterInfo) -> Root<Self> {
        let obj = box WebGpuAdapter {
            reflector_: Reflector::new(),
        };
        reflect_dom_object(obj, window, binding::Wrap)
    }
}

impl binding::WebGpuAdapterMethods for WebGpuAdapter {
    fn GetQueueFamilies(&self) {
        unimplemented!()
    }
}
