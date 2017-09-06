/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{
    WebGpuCommand, WebGpuCommandChan,
    BufferBarrier, ImageBarrier,
    CommandBufferId, CommandBufferInfo, CommandPoolId, SubmitEpoch, SubmitInfo,
    gpu,
};
use dom::bindings::codegen::Bindings::WebGpuCommandBufferBinding as binding;
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding::WebGpuPipelineStage;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::webgpudevice::WebGpuDevice;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpurenderpass::WebGpuRenderpass;
use dom_struct::dom_struct;
use std::cell::Cell;


#[dom_struct]
pub struct WebGpuCommandBuffer {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
    pool_id: CommandPoolId,
    info: CommandBufferInfo,
    submit_epoch: Cell<SubmitEpoch>, //TODO: atomics
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

    pub fn get_id(&self) -> CommandBufferId {
        self.info.id
    }

    pub fn to_submit_info(&self) -> SubmitInfo {
        SubmitInfo {
            pool_id: self.pool_id,
            cb_id: self.info.id,
            submit_epoch: self.submit_epoch.get(),
        }
    }

    fn map_buffer_state(state: binding::WebGpuBufferState) -> gpu::buffer::State {
        let access = gpu::buffer::Access::from_bits(state.access as _).unwrap();
        access
    }

    fn map_image_state(state: binding::WebGpuImageState) -> gpu::image::State {
        let layout = WebGpuDevice::map_image_layout(state.layout);
        let access = gpu::image::Access::from_bits(state.access as _).unwrap();
        (access, layout)
    }

    fn map_rect(rect: &binding::WebGpuRectangle) -> gpu::target::Rect {
        gpu::target::Rect {
            x: rect.x as _,
            y: rect.y as _,
            w: rect.width as _,
            h: rect.height as _,
        }
    }

    fn map_clear_value(cv: binding::WebGpuClearValue) -> gpu::command::ClearValue {
        use self::binding::WebGpuClearValueKind::*;
        use self::gpu::command::{ClearColor, ClearDepthStencil, ClearValue};
        match cv.kind {
            ColorUint => {
                let data = [*cv.data[0] as u32, *cv.data[1] as u32, *cv.data[2] as u32, *cv.data[3] as u32];
                ClearValue::Color(ClearColor::Uint(data))
            }
            ColorInt => {
                let data = [*cv.data[0] as i32, *cv.data[1] as i32, *cv.data[2] as i32, *cv.data[3] as i32];
                ClearValue::Color(ClearColor::Int(data))
            }
            ColorFloat => {
                let data = [*cv.data[0] as f32, *cv.data[1] as f32, *cv.data[2] as f32, *cv.data[3] as f32];
                ClearValue::Color(ClearColor::Float(data))
            }
            DepthStencil => {
                ClearValue::DepthStencil(ClearDepthStencil {
                    depth: *cv.data[0] as f32,
                    stencil: *cv.data[1] as u32,
                })
            }
        }
    }
}

impl binding::WebGpuCommandBufferMethods for WebGpuCommandBuffer {
    fn Begin(&self) {
        //TODO: remember if we are actively recording
        let msg = WebGpuCommand::Begin(self.info.id);
        self.sender.send(msg).unwrap();
    }

    fn Finish(&self) {
        let submit_epoch = self.submit_epoch.get() + 1;
        self.submit_epoch.set(submit_epoch); //TODO

        let msg = WebGpuCommand::Finish(submit_epoch);
        self.sender.send(msg).unwrap();
    }

    fn PipelineBarrier(&self,
        src_stages: WebGpuPipelineStage,
        dst_stages: WebGpuPipelineStage,
        buffers: Vec<binding::WebGpuBufferBarrier>,
        images: Vec<binding::WebGpuImageBarrier>,
    ) {
        let buffer_bars = buffers
            .into_iter()
            .map(|bar| BufferBarrier {
                state_src: Self::map_buffer_state(bar.stateSrc),
                state_dst: Self::map_buffer_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();
        let image_bars = images
            .into_iter()
            .map(|bar| ImageBarrier {
                state_src: Self::map_image_state(bar.stateSrc),
                state_dst: Self::map_image_state(bar.stateDst),
                target: bar.target.get_id(),
            })
            .collect();

        let msg = WebGpuCommand::PipelineBarrier {
            src_stages: gpu::pso::PipelineStage::from_bits(src_stages as _).unwrap(),
            dst_stages: gpu::pso::PipelineStage::from_bits(dst_stages as _).unwrap(),
            buffer_bars,
            image_bars,
        };
        self.sender.send(msg).unwrap();
    }


    fn BeginRenderpass(&self,
        renderpass: &WebGpuRenderpass,
        framebuffer: &WebGpuFramebuffer,
        rect: &binding::WebGpuRectangle,
        clearValues: Vec<binding::WebGpuClearValue>,
    ) {
        let clear_values = clearValues
            .into_iter()
            .map(Self::map_clear_value)
            .collect();

        let msg = WebGpuCommand::BeginRenderpass {
            renderpass: renderpass.get_id(),
            framebuffer: framebuffer.get_id(),
            area: Self::map_rect(rect),
            clear_values,
        };
        self.sender.send(msg).unwrap();
    }

    fn EndRenderpass(&self) {
        let msg = WebGpuCommand::EndRenderpass;
        self.sender.send(msg).unwrap();
    }
}
