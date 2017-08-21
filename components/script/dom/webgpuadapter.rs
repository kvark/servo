/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    AdapterId, AdapterInfo, QueueCount, QueueFamilyId, QueueFamilyInfo, QueueType,
    WebGpuChan, WebGpuMsg, webgpu_channel,
};
use dom::bindings::codegen::Bindings::WebGpuAdapterBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::webgpudevice::WebGpuDevice;
use dom::window::Window;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuAdapter {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    adapter_id: AdapterId,
    queue_families: Vec<QueueFamilyInfo>,
}

impl WebGpuAdapter {
    pub fn new(window: &Window, sender: WebGpuChan, info: AdapterInfo) -> Root<Self> {
        let obj = box WebGpuAdapter {
            reflector_: Reflector::new(),
            sender,
            adapter_id: info.original_id,
            queue_families: info.queue_families,
        };
        reflect_dom_object(obj, window, binding::Wrap)
    }
}

impl binding::WebGpuAdapterMethods for WebGpuAdapter {
    fn GetQueueFamilies(&self) -> Vec<binding::WebGpuQueueFamilyInfo> {
        self.queue_families
            .iter()
            .map(|family| {
                binding::WebGpuQueueFamilyInfo {
                    flags: match family.ty {
                        QueueType::Graphics => 0x7,
                        QueueType::Compute => 0x6,
                        QueueType::Transfer => 0x4,
                    },
                    count: family.count as binding::WebGpuQueueCount,
                    id: family.original_id as binding::WebGpuQueueFamilyId,
                }
            })
            .collect()
    }

    fn Open(&self, queues: Vec<binding::WebGpuRequestedQueues>) -> Root<WebGpuDevice> {
        let queue_families = queues
            .iter()
            .map(|q| (q.id as QueueFamilyId, q.count as QueueCount))
            .collect();

        let (sender, receiver) = webgpu_channel().unwrap();

        self.sender.send(WebGpuMsg::OpenAdapter {
            adapter_id: self.adapter_id,
            queue_families,
            result: sender,
        }).unwrap();

        let device = receiver.recv().unwrap();
        WebGpuDevice::new(&self.global(), self.sender.clone(), device)
    }
}
