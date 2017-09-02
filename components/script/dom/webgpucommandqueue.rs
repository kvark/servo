/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use heapsize::HeapSizeOf;
use canvas_traits::webgpu::{GpuId, QueueId, WebGpuChan, WebGpuMsg,
    gpu, webgpu_channel};
use dom::bindings::codegen::Bindings::WebGpuCommandQueueBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandbuffer::WebGpuCommandBuffer;
use dom::webgpucommandpool::WebGpuCommandPool;
use dom::webgpusemaphore::WebGpuSemaphore;
use dom::webgpufence::WebGpuFence;
use dom_struct::dom_struct;


pub struct LimitsWrapper(gpu::Limits);

impl HeapSizeOf for LimitsWrapper {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[dom_struct]
pub struct WebGpuCommandQueue {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: (GpuId, QueueId),
    limits: LimitsWrapper,
}

impl WebGpuCommandQueue {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        gpu_id: GpuId,
        id: QueueId,
        limits: gpu::Limits,
    ) -> Root<Self> {
        let obj = box WebGpuCommandQueue {
            reflector_: Reflector::new(),
            sender,
            id: (gpu_id, id),
            limits: LimitsWrapper(limits),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn get_id(&self) -> QueueId {
        self.id.1
    }

    pub fn gpu_id(&self) -> GpuId {
        self.id.0
    }

    pub fn get_limits(&self) -> &gpu::Limits {
        &self.limits.0
    }
}

impl binding::WebGpuCommandQueueMethods for WebGpuCommandQueue {
    fn CreateCommandPool(&self) -> Root<WebGpuCommandPool> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateCommandPool {
            gpu_id: self.id.0,
            queue_id: self.id.1,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuCommandPool::new(&self.global(), info)
    }

    fn Submit(&self,
        command_bufs: Vec<Root<WebGpuCommandBuffer>>,
        _waits: Vec<Root<WebGpuSemaphore>>,
        _signals: Vec<Root<WebGpuSemaphore>>,
        fence: Option<&WebGpuFence>,
    ) {
        let msg = WebGpuMsg::Submit {
            gpu_id: self.id.0,
            queue_id: self.id.1,
            command_buffers: command_bufs
                .into_iter()
                .map(|cb| cb.to_submit_info())
                .collect(),
            wait_semaphores: Vec::new(), //TODO
            signal_semaphores: Vec::new(), //TODO
            fence_id: fence.map(|f| f.get_id()),
        };
        self.sender.send(msg).unwrap();
    }
}
