/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan,
    BufferBarrier, ImageBarrier,
    CommandBufferInfo, CommandPoolId, SubmitEpoch, SubmitInfo,
    gpu,
};
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpurenderpass::WebGpuRenderpass;
use dom::webgpusubmit::WebGpuSubmit;
use dom_struct::dom_struct;
use std::cell::Cell;


#[dom_struct]
pub struct WebGpuCommandBuffer {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
    pool_id: CommandPoolId,
    info: CommandBufferInfo,
    submit_epoch: Cell<SubmitEpoch>,
}

impl WebGpuCommandBuffer {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuCommandChan,
        pool_id: CommandPoolId,
        info: CommandBufferInfo,
    ) -> Root<Self> {
        let obj = box WebGpuCommandBuffer {
            reflector_: Reflector::new(),
            sender,
            pool_id,
            info,
            submit_epoch: Cell::new(0),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

fn map_buffer_state(state: binding::WebGpuBufferState) -> gpu::buffer::State {
    let access = gpu::buffer::Access::from_bits(state.access as _).unwrap();
    access
}

fn map_image_state(state: binding::WebGpuImageState) -> gpu::image::State {
    use self::binding::WebGpuImageLayout::*;
    use self::gpu::image::ImageLayout as Il;
    let layout = match state.layout {
        General => Il::General,
        ColorAttachmentOptimal => Il::ColorAttachmentOptimal,
        DepthStencilAttachmentOptimal => Il::DepthStencilAttachmentOptimal,
        DepthStencilReadOnlyOptimal => Il::DepthStencilReadOnlyOptimal,
        ShaderReadOnlyOptimal => Il::ShaderReadOnlyOptimal,
        TransferSrcOptimal => Il::TransferSrcOptimal,
        TransferDstOptimal => Il::TransferDstOptimal,
        Undefined => Il::Undefined,
        Preinitialized => Il::Preinitialized,
        Present => Il::Present,
    };
    let access = gpu::image::Access::from_bits(state.access as _).unwrap();
    (access, layout)
}

impl binding::WebGpuCommandBufferMethods for WebGpuCommandBuffer {
    fn Finish(&self) -> Root<WebGpuSubmit> {
        let submit_epoch = self.submit_epoch.get() + 1;
        self.submit_epoch.set(submit_epoch); //TODO

        let msg = WebGpuCommand::Finish(submit_epoch);
        self.sender.send(msg).unwrap();

        let info = SubmitInfo {
            pool_id: self.pool_id,
            cb_id: self.info.id,
            submit_epoch,
        };
        WebGpuSubmit::new(&self.global(), info)
    }

    fn PipelineBarrier(&self,
        buffer_bars: Vec<binding::WebGpuBufferBarrier>,
        image_bars: Vec<binding::WebGpuImageBarrier>,
    ) {
        let buffers = buffer_bars
            .into_iter()
            .map(|bar| BufferBarrier {
                state_src: map_buffer_state(bar.stateSrc),
                state_dst: map_buffer_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();
        let images = image_bars
            .into_iter()
            .map(|bar| ImageBarrier {
                state_src: map_image_state(bar.stateSrc),
                state_dst: map_image_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();

        let msg = WebGpuCommand::PipelineBarrier(buffers, images);
        self.sender.send(msg).unwrap();
    }


    fn BeginRenderpass(&self,
        renderpass: &WebGpuRenderpass,
        framebuffer: &WebGpuFramebuffer,
        rect: binding::WebGpuRectangle,
        clearValues: Vec<binding::WebGpuClearValue>,
    ) {
        let msg = WebGpuCommand::BeginRenderpass(renderpass.get_id(), framebuffer.get_id());
        self.sender.send(msg).unwrap();
    }

    fn EndRenderpass(&self) {
        let msg = WebGpuCommand::EndRenderpass;
        self.sender.send(msg).unwrap();
    }
}

impl Drop for WebGpuCommandBuffer {
    fn drop(&mut self) {
        let msg = WebGpuCommand::ReturnCommandBuffer(self.info.id);
        self.sender.send(msg).unwrap();
    }
}
