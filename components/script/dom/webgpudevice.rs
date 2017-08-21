/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{DeviceId, DeviceInfo, WebGpuChan};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuDevice {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: DeviceId,
    general_queues: Vec<Root<WebGpuCommandQueue>>,
}

impl WebGpuDevice {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        mut device: DeviceInfo,
    ) -> Root<Self>
    {
        let device_id = device.id;
        let general_queues = device.general
            .drain(..)
            .map(|id| {
                WebGpuCommandQueue::new(global, sender.clone(), id, device_id)
            })
            .collect();
        let obj = box WebGpuDevice {
            reflector_: Reflector::new(),
            sender,
            id: device.id,
            general_queues,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuDeviceMethods for WebGpuDevice {
    fn GeneralQueue(&self) -> Root<WebGpuCommandQueue> {
        self.general_queues[0].clone()
    }
}
