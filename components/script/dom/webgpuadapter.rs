/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    AdapterId, AdapterInfo, QueueCount, QueueFamilyId, QueueFamilyInfo,
    WebGpuChan, WebGpuMsg, hal, webgpu_channel,
};
use dom::bindings::codegen::Bindings::WebGpuAdapterBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::webgpucommandqueue::WebGpuCommandQueue;
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
                        hal::QueueType::General => 0x7,
                        hal::QueueType::Graphics => 0x5,
                        hal::QueueType::Compute => 0x6,
                        hal::QueueType::Transfer => 0x4,
                    },
                    count: family.count as binding::WebGpuQueueCount,
                    id: family.original_id as binding::WebGpuQueueFamilyId,
                }
            })
            .collect()
    }

    fn Open(&self, queues: Vec<binding::WebGpuRequestedQueues>) -> binding::WebGpuGpu {
        let (result, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::OpenAdapter {
            adapter_id: self.adapter_id,
            queue_families: queues
                .iter()
                .map(|q| (q.id as QueueFamilyId, q.count as QueueCount))
                .collect(),
            result,
        };
        self.sender.send(msg).unwrap();

        let gpu = receiver.recv().unwrap();
        let gpu_id = gpu.id;
        let limits = gpu.limits;
        let memory_types = &gpu.mem_types;
        let global = &self.global();
        let sender = self.sender.clone();

        binding::WebGpuGpu {
            generalQueues: gpu.general
                .into_iter()
                .map(|id| {
                    WebGpuCommandQueue::new(global, sender.clone(), gpu_id, id, limits, memory_types)
                })
                .collect(),
            heapTypes: memory_types
                .iter()
                .map(|ht| binding::WebGpuHeapType {
                    id: ht.id as _,
                    properties: ht.properties.bits() as _,
                })
                .collect(),
            device: WebGpuDevice::new(global, sender, gpu_id, limits, memory_types),
        }
    }
}
