/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{GpuId, GpuInfo, WebGpuChan, WebGpuMsg,
    FramebufferDesc, RenderpassDesc, webgpu_channel};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpucommandqueue::WebGpuCommandQueue;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpuimage::WebGpuImage;
use dom::webgpurenderpass::WebGpuRenderpass;
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

    fn CreateRenderpass(&self,
        attachments: Vec<binding::WebGpuAttachmentDesc>,
        subpasses: Vec<binding::WebGpuSubpassDesc>,
    ) -> Root<WebGpuRenderpass>
    {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateRenderpass {
            gpu_id: self.id,
            desc: RenderpassDesc {
                attachments: Vec::new(),
                subpasses: Vec::new(),
                dependencies: Vec::new(),
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuRenderpass::new(&self.global(), info)
    }

    fn CreateFramebuffer(&self,
        renderpass: &WebGpuRenderpass,
        colors: Vec<binding::WebGpuRenderTargetView>,
        depth_stencil: Option<binding::WebGpuDepthStencilView>,
    ) -> Root<WebGpuFramebuffer>
    {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateFramebuffer {
            gpu_id: self.id,
            desc: FramebufferDesc {
                renderpass: renderpass.get_id(),
                colors: Vec::new(),
                depth_stencil: None,
                width: 0,
                height: 0,
                layers: 1,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuFramebuffer::new(&self.global(), info)
    }

    fn ViewImageAsRenderTarget(&self,
        image: &WebGpuImage,
    ) -> binding::WebGpuRenderTargetView
    {
        0
    }
}
