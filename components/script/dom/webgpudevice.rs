/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{DeviceInfo};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuDevice {
    reflector_: Reflector,
}

impl WebGpuDevice {
    pub fn new(global: &GlobalScope, device: DeviceInfo) -> Root<Self> {
        let obj = box WebGpuDevice {
            reflector_: Reflector::new(),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuDeviceMethods for WebGpuDevice {
    fn GeneralQueue(&self) -> Root<WebGpuCommandQueue> {
        WebGpuCommandQueue::new(&self.global())
    }
}
