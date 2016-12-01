use std::{mem, ptr};
use std::sync::Arc;
use vk;
use {Fence, FrameBuffer, FrameClearData, Pipeline,
     RenderPass, ResourceState, Share, TargetView, Texture};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandBuffer {
    inner: vk::CommandBuffer,
    family_index: u32,
    fence: Fence,
}

impl CommandBuffer {
    pub fn new(inner: vk::CommandBuffer, family_id: u32, fence: Fence) -> CommandBuffer {
        CommandBuffer {
            inner: inner,
            family_index: family_id,
            fence: fence,
        }
    }

    pub fn get_fence(&self) -> &Fence {
        &self.fence
    }

    pub fn begin(&self, share: &Share) {
        let info = vk::CommandBufferBeginInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            pNext: ptr::null(),
            flags: 0,
            pInheritanceInfo: ptr::null(),
        };
        assert_eq!(vk::SUCCESS, unsafe {
            share.vk.BeginCommandBuffer(self.inner, &info)
        });
    }

    pub fn end(&self, share: &Share) {
        assert_eq!(vk::SUCCESS, unsafe {
            share.vk.EndCommandBuffer(self.inner)
        });
    }

    pub fn begin_pass(&self, share: &Share, pass: &RenderPass,
                      fb: &FrameBuffer, clear_data: FrameClearData) {
        let mut clears: [vk::ClearValue; 8] = unsafe { mem::zeroed() };
        assert!(clears.len() >= pass.get_num_attachments());
        for i in 0 .. pass.get_num_colors() {
            clears[i] = vk::ClearValue::color(vk::ClearColorValue::float32(clear_data.colors[i]));
        }
        clears[pass.get_num_colors()] = vk::ClearValue::depth_stencil(vk::ClearDepthStencilValue {
            depth: clear_data.depth,
            stencil: clear_data.stencil as u32,
        });

        let info = vk::RenderPassBeginInfo {
            sType: vk::STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            pNext: ptr::null(),
            renderPass: pass.inner,
            framebuffer: fb.inner,
            renderArea: vk::Rect2D {
                offset: vk::Offset2D {
                    x: 0,
                    y: 0,
                },
                extent: fb.dim.clone().into(),
            },
            clearValueCount: pass.get_num_attachments() as u32,
            pClearValues: clears.as_ptr(),
        };
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            height: fb.dim.h as f32,
            width: fb.dim.w as f32,
            minDepth: 0.0,
            maxDepth: 1.0,
        };
        let scissor_rect = vk::Rect2D {
            offset: vk::Offset2D {
                x: 0,
                y: 0,
            },
            extent: fb.dim.clone().into(),
        };
        unsafe {
            share.vk.CmdBeginRenderPass(self.inner, &info, vk::SUBPASS_CONTENTS_INLINE);
            share.vk.CmdSetViewport(self.inner, 0, 1, &viewport);
            share.vk.CmdSetScissor(self.inner, 0, 1, &scissor_rect);
        }
    }

    pub fn end_pass(&self, share: &Share) {
        unsafe {
            share.vk.CmdEndRenderPass(self.inner);
        }
    }

    pub fn bind_pipeline(&self, share: &Share, pipeline: &Pipeline) {
        unsafe {
            share.vk.CmdBindPipeline(self.inner, vk::PIPELINE_BIND_POINT_GRAPHICS, pipeline.get_inner());
        }
    }

    pub fn draw(&self, share: &Share, start: u32, count: u32, instances: u32) {
        unsafe {
            share.vk.CmdDraw(self.inner, count, instances, start, 0);
        }
    }

    fn set_image_layout(&self, share: &Share, res: &mut ResourceState,
                        texture: &Arc<Texture>, layer: u32, layout: vk::ImageLayout) {
        let key = (texture.clone(), layer);
        let old_layout = *res.image_layouts.get(&key)
                                           .unwrap_or(&texture.default_layout);
        if layout != old_layout {
            self.image_barrier(share, &texture, layer, old_layout, layout);
            res.image_layouts.insert(key, layout);
        }
    }

    pub fn reset_state(&self, share: &Share, res: &mut ResourceState) {
        for ((texture, layer), layout) in res.image_layouts.drain() {
            if layout != texture.default_layout {
                self.image_barrier(share, &texture, layer, layout, texture.default_layout)
            }
        }
    }

    fn image_barrier(&self, share: &Share, texture: &Texture, layer: u32,
                     old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
        let mut access = 0; //TODO
        if texture.usage & vk::IMAGE_USAGE_TRANSFER_SRC_BIT != 0 {
            access |= vk::ACCESS_TRANSFER_WRITE_BIT;
        }
        if texture.usage & vk::IMAGE_USAGE_TRANSFER_DST_BIT != 0 {
            access |= vk::ACCESS_TRANSFER_READ_BIT;
        }
        if texture.usage & vk::IMAGE_USAGE_SAMPLED_BIT != 0 {
            access |= vk::ACCESS_SHADER_READ_BIT;
        }
        let barrier = vk::ImageMemoryBarrier {
            sType: vk::STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            pNext: ptr::null(),
            srcAccessMask: access,
            dstAccessMask: access,
            oldLayout: old_layout,
            newLayout: new_layout,
            srcQueueFamilyIndex: self.family_index,
            dstQueueFamilyIndex: self.family_index,
            image: texture.inner,
            subresourceRange: vk::ImageSubresourceRange {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                baseMipLevel: 0,
                levelCount: 1,
                baseArrayLayer: layer,
                layerCount: 1,
            },
        };
        unsafe {
            share.vk.CmdPipelineBarrier(self.inner,
                vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, vk::PIPELINE_STAGE_TOP_OF_PIPE_BIT, 0,
                0, ptr::null(), 0, ptr::null(), 1, &barrier);
        }
    }

    pub fn clear_color(&self, share: &Share, res: &mut ResourceState,
                       view: &TargetView, color: [f32; 4]) {
        let layout = vk::IMAGE_LAYOUT_GENERAL;
        self.set_image_layout(share, res, &view.texture, view.layer, layout);

        let value = vk::ClearColorValue::float32(color);
        let range = vk::ImageSubresourceRange {
            aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: view.layer,
            layerCount: 1,
        };
        unsafe {
            share.vk.CmdClearColorImage(self.inner,
                                        view.texture.inner,
                                        layout,
                                        &value, 1, &range);
        }
    }

    pub fn copy_texture(&self, share: &Share, res: &mut ResourceState,
                        src: &Arc<Texture>, src_layer: u32,
                        dst: &Arc<Texture>, dst_layer: u32) {
        assert_eq!(src.dim, dst.dim);
        let src_layout = vk::IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
        let dst_layout = vk::IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
        self.set_image_layout(share, res, src, src_layer, src_layout);
        self.set_image_layout(share, res, dst, dst_layer, dst_layout);

        let regions = [vk::ImageCopy {
            srcSubresource: vk::ImageSubresourceLayers {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: src_layer,
                layerCount: 1,
            },
            srcOffset: vk::Offset3D {
                x: 0, y: 0, z: 0,
            },
            dstSubresource: vk::ImageSubresourceLayers {
                aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
                mipLevel: 0,
                baseArrayLayer: dst_layer,
                layerCount: 1,
            },
            dstOffset: vk::Offset3D {
                x: 0, y: 0, z: 0,
            },
            extent: vk::Extent3D {
                width: src.dim.w,
                height: src.dim.h,
                depth: src.dim.d,
            },
        }];
        unsafe {
            share.vk.CmdCopyImage(self.inner,
                                  src.inner, src_layout,
                                  dst.inner, dst_layout,
                                  regions.len() as u32,
                                  regions.as_ptr());
        }
    }
}

pub struct CommandPool {
    inner: vk::CommandPool,
    family_index: u32,
}

impl CommandPool {
    pub fn new(inner: vk::CommandPool, family_id: u32) -> CommandPool {
        CommandPool {
            inner: inner,
            family_index: family_id,
        }
    }

    pub fn get_family_id(&self) -> u32 {
        self.family_index
    }

    pub fn get_inner(&self) -> vk::CommandPool {
        self.inner
    }
}

pub struct Queue {
    inner: vk::Queue,
    family_index: u32,
}

impl Queue {
    pub fn new(inner: vk::Queue, family_id: u32) -> Queue {
        Queue {
            inner: inner,
            family_index: family_id,
        }
    }

    pub fn get_family_id(&self) -> u32 {
        self.family_index
    }

    pub fn submit(&self, share: &Share, com: &CommandBuffer) {
        assert_eq!(self.family_index, com.family_index);
        com.end(share);

        let info = vk::SubmitInfo {
            sType: vk::STRUCTURE_TYPE_SUBMIT_INFO,
            commandBufferCount: 1,
            pCommandBuffers: &com.inner,
            .. unsafe { mem::zeroed() }
        };
        let fence = com.fence.get_inner();

        assert_eq!(vk::SUCCESS, unsafe {
            share.vk.QueueSubmit(self.inner, 1, &info, fence)
        });
    }
}
