/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{GpuId, GpuInfo, WebGpuChan};
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
    id: GpuId,
    general_queues: Vec<Root<WebGpuCommandQueue>>,
}

impl WebGpuDevice {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        gpu: GpuInfo,
    ) -> Root<Self>
    {
        let gpu_id = gpu.id;
        let general_queues = gpu.general
            .into_iter()
            .map(|id| {
                WebGpuCommandQueue::new(global, sender.clone(), gpu_id, id)
            })
            .collect();
        let obj = box WebGpuDevice {
            reflector_: Reflector::new(),
            sender,
            id: gpu_id,
            general_queues,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuDeviceMethods for WebGpuDevice {
    fn GeneralQueue(&self) -> Root<WebGpuCommandQueue> {
        self.general_queues[0].clone()
    }

    fn CreateRenderPass(&self,
        attachments: Vec<binding::WebGpuAttachmentDesc>,
        subpasses: Vec<binding::WebGpuSubpassDesc>,
    ) -> binding::WebGpuRenderpass
    {
        0
    }

    fn CreateFramebuffer(&self,
        colors: Vec<binding::WebGpuRenderTargetView>,
    ) -> binding::WebGpuFramebuffer
    {
        0
    }

    fn ViewImageAsRenderTarget(&self,
        image: binding::WebGpuImage,
    ) -> binding::WebGpuRenderTargetView
    {
        0
    }
}
