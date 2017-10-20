/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use webgpu::backend;
use webgpu::hal::{self,
    Adapter, DescriptorPool, Device, Instance, QueueFamily,
    RawCommandBuffer, RawCommandPool, RawCommandQueue,
};

use std::thread;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use webgpu_mode::{LazyVec, ResourceHub};
/// WebGL Threading API entry point that lives in the constellation.
/// It allows to get a WebGpuThread handle for each script pipeline.
pub use ::webgpu_mode::WebGpuThreads;

use euclid::Size2D;
use webrender_api as wrapi;


enum PoolCommand<B: hal::Backend> {
    FinishBuffer(w::CommandBufferId, w::SubmitEpoch, B::CommandBuffer),
    Reset,
    Destroy,
}

struct CommandPoolHandle<B: hal::Backend> {
    _join: thread::JoinHandle<()>,
    //Note: you can't have more than one buffer encoded at a single time,
    // but you can have multiple finished command buffers ready for submission.
    submits: HashMap<w::CommandBufferId, (w::SubmitEpoch, B::CommandBuffer)>,
    receiver: mpsc::Receiver<PoolCommand<B>>,
    is_alive: bool,
}

impl<B: hal::Backend> CommandPoolHandle<B> {
    fn process_command(&mut self, command: PoolCommand<B>) {
        match command {
            PoolCommand::FinishBuffer(cb_id, submit_epoch, cb) => {
                self.submits.insert(cb_id, (submit_epoch, cb));
            }
            PoolCommand::Reset => {
                self.submits.clear();
            }
            PoolCommand::Destroy => {
                //self.join.join(
                self.is_alive = false;
            }
        }
    }

    fn check_commands(&mut self) {
        while let Ok(command) = self.receiver.try_recv() {
            self.process_command(command);
        }
    }

    fn extract_submit(&mut self,
        cb_id: w::CommandBufferId,
        submit_epoch: w::SubmitEpoch,
    ) -> B::CommandBuffer
    {
        loop {
            if let Some(value) = self.submits.remove(&cb_id) {
                match value.0.cmp(&submit_epoch) {
                    Ordering::Less => {
                        warn!("Skipping submission epoch {:?}", value.0);
                    }
                    Ordering::Greater => {
                        panic!("Stale submission epoch {:?}", value.0);
                    }
                    Ordering::Equal => {
                        return value.1
                    }
                }
            }
            let command = self.receiver.recv().unwrap();
            self.process_command(command);
        }
    }
}


struct Memory<B: hal::Backend> {
    raw: B::Memory,
    size: usize,
}

pub struct WebGpuThread<B: hal::Backend> {
    /// Channel used to generate/update or delete `wrapi::ImageKey`s.
    webrender_api: wrapi::RenderApi,
    present_chan: w::WebGpuPresentChan,
    adapters: Vec<B::Adapter>,
    memories: LazyVec<Memory<B>>,
    rehub: Arc<ResourceHub<B>>,
    command_pools: LazyVec<CommandPoolHandle<B>>,
}

impl WebGpuThread<backend::Backend> {
    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start(
        webrender_api_sender: wrapi::RenderApiSender,
        present_chan: w::WebGpuPresentChan,
        rehub: Arc<ResourceHub<backend::Backend>>,
    ) -> (w::WebGpuSender<w::WebGpuMsg>, wrapi::IdNamespace) {
        let webrender_api = webrender_api_sender.create_api();
        let namespace = webrender_api.get_namespace_id();
        let (sender, receiver) = w::webgpu_channel::<w::WebGpuMsg>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGpuThread".to_owned()).spawn(move || {
            let instance = backend::Instance::create("Servo", 1);
            let mut renderer: Self = WebGpuThread {
                webrender_api,
                present_chan,
                adapters: instance.enumerate_adapters(),
                memories: LazyVec::new(),
                rehub,
                command_pools: LazyVec::new(),
            };
            let webgpu_chan = sender;
            loop {
                renderer.process_pool_commands();
                let msg = receiver.recv().unwrap();
                let exit = renderer.handle_msg(msg, &webgpu_chan);
                if exit {
                    return;
                }
            }
        }).expect("Thread spawning failed");

        (result, namespace)
    }

    #[cfg(windows)]
    fn create_shader_module_hlsl(
        &mut self,
        gpu_id: w::GpuId,
        stage: hal::pso::Stage,
        data: Vec<u8>,
    ) -> w::ShaderModuleInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let entry = match stage {
            hal::pso::Stage::Vertex => "vs_main",
            hal::pso::Stage::Fragment => "fs_main",
            _ => unimplemented!()
        };
        let module = gpu.device
            .create_shader_module_from_source(stage, entry, "main", &data)
            .unwrap();

        w::ShaderModuleInfo {
            id: self.rehub.shaders.write().unwrap().push(module),
        }
    }

    #[cfg(target_os = "macos")]
    fn create_shader_module_msl(
        &mut self,
        gpu_id: w::GpuId,
        data: String,
    ) -> w::ShaderModuleInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let module = gpu.device
            .create_shader_library_from_source(&data, backend::LanguageVersion::new(2, 0))
            .unwrap();

        w::ShaderModuleInfo {
            id: self.rehub.shaders.write().unwrap().push(module),
        }
    }

    //TODO: make backend-agnostic (requires getting rid of HLSL path)
    /// Handles a generic WebGpuMsg message
    fn handle_msg(&mut self, msg: w::WebGpuMsg, webgpu_chan: &w::WebGpuChan) -> bool {
        debug!("got message {:?}", msg);
        match msg {
            w::WebGpuMsg::CreateContext { size, external_image_id, result } => {
                let info = self
                    .create_context(size, external_image_id)
                    .map(|(presenter, adapters, image_key)| w::ContextInfo {
                        presenter,
                        adapters,
                        sender: webgpu_chan.clone(),
                        image_key,
                    });
                result.send(info).unwrap();
            }
            w::WebGpuMsg::OpenAdapter { adapter_id, queue_families, result } => {
                use std::mem;
                let adapter = &mut self.adapters[adapter_id as usize];
                let all_families = adapter.get_queue_families();
                let families = queue_families
                    .iter()
                    .map(|&(id, count)| {
                        let (ref family, type_) = all_families[id as usize];
                        (family, type_, count as u32)
                    })
                    .collect::<Vec<_>>();
                let mut gpu = adapter.open(&families);
                let general_queues = (0 .. gpu.general_queues.len() as w::QueueId).collect();
                let info = w::GpuInfo {
                    limits: gpu.device.get_limits().clone(),
                    mem_types: mem::replace(&mut gpu.memory_types, Vec::new()),
                    id: self.rehub.gpus.lock().unwrap().push(gpu),
                    general: general_queues,
                };
                result.send(info).unwrap();
            }
            w::WebGpuMsg::CreateCommandPool { gpu_id, queue_id, flags, result } => {
                let command_pool = self.create_command_pool(gpu_id, queue_id, flags);
                result.send(command_pool).unwrap();
            }
            w::WebGpuMsg::Submit { gpu_id, queue_id, command_buffers, fence_id, feedback, .. } => {
                self.submit(gpu_id, queue_id, command_buffers, fence_id);
                if let Some(sender) = feedback {
                    sender.send(()).unwrap();
                }
            }
            w::WebGpuMsg::Present { image_key, external_image_id, size, stride } => {
                let desc = wrapi::ImageDescriptor {
                    format: wrapi::ImageFormat::BGRA8,
                    width: size.width,
                    height: size.height,
                    stride: Some(stride),
                    offset: 0,
                    is_opaque: true,
                };
                let data = wrapi::ImageData::External(wrapi::ExternalImageData {
                    id: external_image_id,
                    channel_index: 0,
                    image_type: wrapi::ExternalImageType::ExternalBuffer,
                });

                let mut updates = wrapi::ResourceUpdates::new();
                updates.update_image(image_key, desc, data, None);
                self.webrender_api.update_resources(updates);
            }
            w::WebGpuMsg::Exit => {
                return true;
            }
            w::WebGpuMsg::CreateFence { gpu_id, set, result } => {
                let fence = self.create_fence(gpu_id, set);
                debug!("fence: created {:?} with set {}", fence, set);
                result.send(fence).unwrap();
            }
            w::WebGpuMsg::ResetFences { gpu_id, fence_ids } => {
                debug!("fence: resetting {:?}", fence_ids);
                let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
                let store = self.rehub.fences.read().unwrap();
                let fences_raw = fence_ids
                    .into_iter()
                    .map(|f| &store[f])
                    .collect::<Vec<_>>();
                gpu.device.reset_fences(&fences_raw);
            }
            w::WebGpuMsg::WaitForFences { gpu_id, fence_ids, mode, timeout, result } => {
                debug!("fence: waiting for {:?} with timeout {}", fence_ids, timeout);
                let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
                let store = self.rehub.fences.read().unwrap();
                let fences_raw = fence_ids
                    .into_iter()
                    .map(|f| &store[f])
                    .collect::<Vec<_>>();

                let done = gpu.device.wait_for_fences(&fences_raw, mode, timeout);
                result.send(done).unwrap();
            }
            w::WebGpuMsg::AllocateMemory { gpu_id, desc, result } => {
                let mem = self.allocate_memory(gpu_id, desc);
                result.send(mem).unwrap();
            }
            w::WebGpuMsg::CreateBuffer { gpu_id, desc, result } => {
                let buffer = self.create_buffer(gpu_id, desc);
                result.send(buffer).unwrap();
            }
            w::WebGpuMsg::CreateImage { gpu_id, desc, result } => {
                let image = self.create_image(gpu_id, desc);
                result.send(image).unwrap();
            }
            w::WebGpuMsg::CreateFramebuffer { gpu_id, desc, result } => {
                let framebuffer = self.create_framebuffer(gpu_id, desc);
                result.send(framebuffer).unwrap();
            }
            w::WebGpuMsg::CreateRenderPass { gpu_id, desc, result } => {
                let render_pass = self.create_renderpass(gpu_id, desc);
                result.send(render_pass).unwrap();
            }
            w::WebGpuMsg::CreateDescriptorSetLayout { gpu_id, bindings, result } => {
                let layout = self.create_descriptor_set_layout(gpu_id, bindings);
                result.send(layout).unwrap();
            }
            w::WebGpuMsg::CreatePipelineLayout { gpu_id, set_layout_ids, result } => {
                let layout = self.create_pipeline_layout(gpu_id, set_layout_ids);
                result.send(layout).unwrap();
            }
            w::WebGpuMsg::CreateDescriptorPool { gpu_id, max_sets, ranges, result } => {
                let pool = self.create_descriptor_pool(gpu_id, max_sets, ranges);
                result.send(pool).unwrap();
            }
            w::WebGpuMsg::AllocateDescriptorSets { pool_id, set_layout_ids, result } => {
                let sets = self.allocate_descriptor_sets(pool_id, set_layout_ids);
                let set_store = &mut self.rehub.descriptors.write().unwrap();
                for set in sets {
                    let info = w::DescriptorSetInfo {
                        id: set_store.push(set),
                    };
                    result.send(info).unwrap();
                }
            }
            w::WebGpuMsg::CreateShaderModule { gpu_id, data, result } => {
                let module = self.create_shader_module(gpu_id, data);
                result.send(module).unwrap();
            }
            #[cfg(windows)]
            w::WebGpuMsg::CreateShaderModuleHLSL { gpu_id, stage, data, result } => {
                let module = self.create_shader_module_hlsl(gpu_id, stage, data);
                result.send(module).unwrap();
            }
            #[cfg(target_os = "macos")]
            w::WebGpuMsg::CreateShaderModuleMSL { gpu_id, data, result } => {
                let module = self.create_shader_module_msl(gpu_id, data);
                result.send(module).unwrap();
            }
            w::WebGpuMsg::CreateGraphicsPipelines { gpu_id, descriptors, result } => {
                let pipelines = self.create_graphics_pipelines(gpu_id, descriptors);
                let mut pso_store = self.rehub.graphics_pipes.write().unwrap();
                for pso in pipelines {
                    let info = w::GraphicsPipelineInfo {
                        id: pso_store.push(pso.unwrap()),
                    };
                    result.send(info).unwrap();
                }
            }
            w::WebGpuMsg::CreateSampler { gpu_id, desc, result } => {
                let sampler = self.create_sampler(gpu_id, desc);
                result.send(sampler).unwrap();
            }
            w::WebGpuMsg::CreateImageView { gpu_id, image_id, format, range, result } => {
                let view = self.create_image_view(gpu_id, image_id, format, range);
                result.send(view).unwrap();
            }
            w::WebGpuMsg::UploadBufferData { gpu_id, buffer_id, data } => {
                let device = &mut self.rehub.gpus.lock().unwrap()[gpu_id].device;
                let buffer = &self.rehub.buffers.read().unwrap()[buffer_id];
                let mut writer = device.acquire_mapping_writer::<u8>(buffer, 0 .. data.len() as u64)
                    .unwrap();
                writer.copy_from_slice(&data);
                device.release_mapping_writer(writer);
            }
            w::WebGpuMsg::UpdateDescriptorSets { gpu_id, writes } => {
                self.update_descriptor_sets(gpu_id, writes);
            }
        }

        false
    }
}

impl<B: hal::Backend> WebGpuThread<B> {
    /// Creates a new WebGpuContext
    fn create_context(&mut self,
        size: Size2D<u32>,
        external_image_id: wrapi::ExternalImageId,
    ) -> Result<(w::Presenter, Vec<w::AdapterInfo>, wrapi::ImageKey), String> {
        let adapters = self.adapters
            .iter()
            .enumerate()
            .map(|(aid, ad)| {
                let queue_families = ad
                    .get_queue_families()
                    .iter()
                    .enumerate()
                    .map(|(qid, &(ref family, ty))| {
                        w::QueueFamilyInfo {
                            ty,
                            count: family.num_queues() as _,
                            original_id: qid as _,
                        }
                    })
                    .collect();
                w::AdapterInfo {
                    info: ad.get_info().clone(),
                    queue_families,
                    original_id: aid as _,
                }
            })
            .collect();

        let image_key = self.webrender_api.generate_image_key();
        {
            let desc = wrapi::ImageDescriptor {
                format: wrapi::ImageFormat::BGRA8,
                width: size.width,
                height: size.height,
                stride: None,
                offset: 0,
                is_opaque: true,
            };

            let data = if false { // raw pixels?
                let pixels = (0..size.width*size.height*4).map(|_| 0u8).collect();
                wrapi::ImageData::Raw(Arc::new(pixels))
            } else {
                wrapi::ImageData::External(wrapi::ExternalImageData {
                    id: external_image_id,
                    channel_index: 0,
                    image_type: wrapi::ExternalImageType::ExternalBuffer,
                })
            };

            let mut updates = wrapi::ResourceUpdates::new();
            updates.add_image(image_key, desc, data, None);
            self.webrender_api.update_resources(updates);
        };

        let presenter = w::Presenter {
            id: external_image_id,
            channel: self.present_chan.clone(),
        };

        Ok((presenter, adapters, image_key))
    }

    #[allow(unsafe_code)]
    fn create_command_pool(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
        flags: hal::pool::CommandPoolCreateFlags,
    ) -> w::CommandPoolInfo
    where
        B::Device: Send,
        B::CommandQueue: Send,
        B::DescriptorSetLayout: Send + Sync,
        B::DescriptorPool: Send + Sync,
    {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let queue = gpu.general_queues[queue_id as usize].as_raw();//TODO
        let pool = unsafe {
            B::CommandPool::from_queue(queue, flags)
        };

        let (channel, receiver) = w::webgpu_channel().unwrap();
        let (int_sender, int_receiver) = mpsc::channel();
        let rehub = self.rehub.clone();

        let join = thread::spawn(move|| {
            Self::run_command_thread(receiver, int_sender, pool, rehub)
        });
        let handle = CommandPoolHandle {
            _join: join,
            submits: HashMap::new(),
            receiver: int_receiver,
            is_alive: true,
        };

        w::CommandPoolInfo {
            channel,
            id: self.command_pools.push(handle),
        }
    }

    #[allow(unsafe_code)]
    fn run_command_thread(
        receiver: w::WebGpuReceiver<w::WebGpuCommand>,
        channel: mpsc::Sender<PoolCommand<B>>,
        mut pool: B::CommandPool,
        rehub: Arc<ResourceHub<B>>,
    ) {
        let mut com_buffers = LazyVec::new();
        let mut active_id = None;

        while let Ok(com) = receiver.recv() {
            debug!("got command {:?}", com);
            match com {
                w::WebGpuCommand::Reset => {
                    debug_assert_eq!(active_id, None);
                    pool.reset();
                    channel.send(PoolCommand::Reset).unwrap();
                }
                w::WebGpuCommand::Exit => {
                    debug_assert_eq!(active_id, None);
                    channel.send(PoolCommand::Destroy).unwrap();
                    return
                }
                w::WebGpuCommand::AllocateCommandBuffers(count, result) => {
                    let cbufs = pool.allocate(count as _);

                    for cb in cbufs {
                        let info = w::CommandBufferInfo {
                            id: com_buffers.push(cb),
                        };
                        result.send(info).unwrap();
                    }
                }
                w::WebGpuCommand::FreeCommandBuffers(cb_ids) => {
                    let cbufs = cb_ids
                        .into_iter()
                        .map(|id| {
                            debug_assert_ne!(active_id, Some(id));
                            com_buffers.remove(id).unwrap()
                        })
                        .collect::<Vec<_>>();

                    //TODO: notify the gpu thread?
                    unsafe {
                        pool.free(cbufs)
                    };
                }
                w::WebGpuCommand::Begin(id) => {
                    debug_assert_eq!(active_id, None);
                    active_id = Some(id);
                    com_buffers[id].begin();
                }
                w::WebGpuCommand::Finish(submit_epoch) => {
                    //TODO: check cb epoch
                    let id = active_id.take().unwrap();
                    com_buffers[id].finish();
                    let submit = com_buffers[id].clone();
                    let cmd = PoolCommand::FinishBuffer(id, submit_epoch, submit);
                    channel.send(cmd).unwrap();
                }
                w::WebGpuCommand::PipelineBarrier { stages, buffer_bars, image_bars } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let buffer_store = rehub.buffers.read().unwrap();
                    let image_store = rehub.images.read().unwrap();

                    let buffer_iter = buffer_bars
                        .into_iter()
                        .map(|bar| hal::memory::Barrier::Buffer {
                            states: bar.states,
                            target: &buffer_store[bar.target],
                            //range: 0..1,
                        });

                    let image_iter = image_bars
                        .into_iter()
                        .map(|bar| hal::memory::Barrier::Image {
                            states: bar.states,
                            target: &image_store[bar.target],
                            range: hal::image::SubresourceRange {
                                aspects: hal::image::ASPECT_COLOR,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        });

                    let barriers = buffer_iter
                        .chain(image_iter)
                        .collect::<Vec<_>>();
                    cb.pipeline_barrier(stages, &barriers);
                }
                w::WebGpuCommand::BeginRenderPass { render_pass, framebuffer, area, clear_values } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let pass = &rehub.render_passes.read().unwrap()[render_pass];
                    let fbo = &rehub.framebuffers.read().unwrap()[framebuffer];
                    cb.begin_renderpass(pass, fbo, area, &clear_values, hal::command::SubpassContents::Inline);
                }
                w::WebGpuCommand::EndRenderPass => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    cb.end_renderpass();
                }
                w::WebGpuCommand::CopyBufferToImage { source_id, dest_id, dest_layout, regions } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let source = &rehub.buffers.read().unwrap()[source_id];
                    let destination = &rehub.images.read().unwrap()[dest_id];

                    cb.copy_buffer_to_image(source, destination, dest_layout, &regions);
                }
                w::WebGpuCommand::CopyImageToBuffer { source_id, source_layout, dest_id, regions } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let source = &rehub.images.read().unwrap()[source_id];
                    let destination = &rehub.buffers.read().unwrap()[dest_id];

                    cb.copy_image_to_buffer(source, source_layout, destination, &regions);
                }
                w::WebGpuCommand::BindGraphicsPipeline(pso_id) => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let pso = &rehub.graphics_pipes.read().unwrap()[pso_id];
                    cb.bind_graphics_pipeline(pso);
                }
                w::WebGpuCommand::BindGraphicsDescriptorSets { layout_id, desc_offset, set_ids } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let layout = &rehub.pipe_layouts.read().unwrap()[layout_id];
                    let desc_store = rehub.descriptors.read().unwrap();

                    let sets = set_ids
                        .into_iter()
                        .map(|id| &desc_store[id])
                        .collect::<Vec<_>>();

                    cb.bind_graphics_descriptor_sets(layout, desc_offset, &sets);
                }
                w::WebGpuCommand::SetScissors(rects) => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    cb.set_scissors(&rects);
                }
                w::WebGpuCommand::SetViewports(viewports) => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    cb.set_viewports(&viewports);
                }
                w::WebGpuCommand::Draw(vertices, instances) => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    cb.draw(vertices, instances);
                }
            }
        }
    }

    #[allow(unsafe_code)]
    fn submit(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
        command_buffers: Vec<w::SubmitInfo>,
        fence_id: Option<w::FenceId>,
    ) {
        let cmd_buffers = command_buffers
            .into_iter()
            .map(|info| {
                self.command_pools[info.pool_id]
                    .extract_submit(info.cb_id, info.submit_epoch)
            })
            .collect::<Vec<_>>();

        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let fence_store = &self.rehub.fences.read().unwrap();
        let queue = gpu.general_queues[queue_id as usize].as_mut();

        let submission = hal::RawSubmission {
            cmd_buffers: &cmd_buffers,
            wait_semaphores: &[],
            signal_semaphores: &[],
        };
        let fence = fence_id.map(|id| &fence_store[id]);

        unsafe {
            queue.submit_raw(submission, fence)
        };
    }

    fn process_pool_commands(&mut self) {
        self.command_pools.retain(|pool| {
            pool.check_commands();
            pool.is_alive
        });
    }

    fn create_fence(&mut self,
        gpu_id: w::GpuId,
        set: bool,
    ) -> w::FenceId {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let fence = gpu.device.create_fence(set);
        self.rehub.fences.write().unwrap().push(fence)
    }

    fn allocate_memory(&mut self,
        gpu_id: w::GpuId,
        desc: w::MemoryDesc,
    ) -> w::MemoryInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let raw = gpu.device.allocate_memory(
            &desc.ty,
            desc.size as _,
        ).unwrap();
        let mem = Memory {
            raw,
            size: desc.size,
        };

        w::MemoryInfo {
            id: self.memories.push(mem),
        }
    }

    fn create_buffer(&mut self,
        gpu_id: w::GpuId,
        desc: w::BufferDesc,
    ) -> w::BufferInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let mem = &self.memories[desc.mem_id];

        let unbound = gpu.device
            .create_buffer(desc.size as _, desc.stride as _, desc.usage)
            .unwrap();
        let requirements = gpu.device.get_buffer_requirements(&unbound);
        debug_assert_ne!(requirements.alignment, 0);

        let offset = (desc.mem_offset as u64 + requirements.alignment - 1) &
            !(requirements.alignment - 1);
        assert!(offset + requirements.size <= mem.size as u64);
        let buffer = gpu.device
            .bind_buffer_memory(&mem.raw, offset, unbound)
            .unwrap();

        w::BufferInfo {
            id: self.rehub.buffers.write().unwrap().push(buffer),
            occupied_size: (offset + requirements.size) as usize - desc.mem_offset,
        }
    }

    fn create_image(&mut self,
        gpu_id: w::GpuId,
        desc: w::ImageDesc,
    ) -> w::ImageInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let mem = &self.memories[desc.mem_id];

        let unbound = gpu.device
            .create_image(desc.kind, desc.levels, desc.format, desc.usage)
            .unwrap();
        let requirements = gpu.device.get_image_requirements(&unbound);
        debug_assert_ne!(requirements.alignment, 0);

        let offset = (desc.mem_offset as u64 + requirements.alignment - 1) &
            !(requirements.alignment - 1);
        assert!(offset + requirements.size <= mem.size as u64);
        let image = gpu.device
            .bind_image_memory(&mem.raw, offset, unbound)
            .unwrap();

        w::ImageInfo {
            id: self.rehub.images.write().unwrap().push(image),
            occupied_size: (offset + requirements.size) as usize - desc.mem_offset,
        }
    }

    fn create_framebuffer(&mut self,
        gpu_id: w::GpuId,
        desc: w::FramebufferDesc,
    ) -> w::FramebufferInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let render_pass = &self.rehub.render_passes.read().unwrap()[desc.render_pass];
        let iv_store = self.rehub.image_views.read().unwrap();
        let attachments = desc.attachments
            .into_iter()
            .map(|id| &iv_store[id])
            .collect::<Vec<_>>();
        let fbo = gpu.device.create_framebuffer(
            render_pass,
            &attachments,
            desc.extent,
        ).unwrap();

        w::FramebufferInfo {
            id: self.rehub.framebuffers.write().unwrap().push(fbo),
        }
    }

    fn create_renderpass(&mut self,
        gpu_id: w::GpuId,
        desc: w::RenderPassDesc,
    ) -> w::RenderPassInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let subpasses = desc.subpasses
            .iter()
            .map(|sp| hal::pass::SubpassDesc {
                colors: &sp.colors,
                depth_stencil: None,
                inputs: &[], //TODO
                preserves: &[], //TODO
            })
            .collect::<Vec<_>>();
        let rp = gpu.device.create_render_pass(
            &desc.attachments,
            &subpasses,
            &desc.dependencies,
        );

        w::RenderPassInfo {
            id: self.rehub.render_passes.write().unwrap().push(rp),
        }
    }

    fn create_descriptor_set_layout(
        &mut self,
        gpu_id: w::GpuId,
        bindings: Vec<hal::pso::DescriptorSetLayoutBinding>,
    ) -> w::DescriptorSetLayoutInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let layout = gpu.device.create_descriptor_set_layout(&bindings);

        w::DescriptorSetLayoutInfo {
            id: self.rehub.set_layouts.write().unwrap().push(layout)
        }
    }

    fn create_pipeline_layout(
        &mut self,
        gpu_id: w::GpuId,
        set_layout_ids: Vec<w::DescriptorSetLayoutId>,
    ) -> w::PipelineLayoutInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let set_layout_store = self.rehub.set_layouts.read().unwrap();

        let set_layouts = set_layout_ids
            .into_iter()
            .map(|id| &set_layout_store[id])
            .collect::<Vec<_>>();
        let layout = gpu.device.create_pipeline_layout(&set_layouts);

        w::PipelineLayoutInfo {
            id: self.rehub.pipe_layouts.write().unwrap().push(layout),
        }
    }

    fn create_descriptor_pool(
        &mut self,
        gpu_id: w::GpuId,
        max_sets: usize,
        ranges: Vec<hal::pso::DescriptorRangeDesc>,
    ) -> w::DescriptorPoolInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let pool = gpu.device.create_descriptor_pool(max_sets, &ranges);

        w::DescriptorPoolInfo {
            id: self.rehub.pools.lock().unwrap().push(pool),
        }
    }

    fn allocate_descriptor_sets(
        &mut self,
        pool_id: w::DescriptorPoolId,
        set_layout_ids: Vec<w::DescriptorSetLayoutId>,
    ) -> Vec<B::DescriptorSet> {
        let pool = &mut self.rehub.pools.lock().unwrap()[pool_id];
        let set_layout_store = self.rehub.set_layouts.read().unwrap();

        let set_layouts = set_layout_ids
            .into_iter()
            .map(|id| &set_layout_store[id])
            .collect::<Vec<_>>();

        pool.allocate_sets(&set_layouts)
    }

    fn create_shader_module(
        &mut self,
        gpu_id: w::GpuId,
        data: Vec<u8>,
    ) -> w::ShaderModuleInfo {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];

        let module = gpu.device.create_shader_module(&data).unwrap();

        w::ShaderModuleInfo {
            id: self.rehub.shaders.write().unwrap().push(module),
        }
    }

    fn create_graphics_pipelines(
        &mut self,
        gpu_id: w::GpuId,
        descriptors: Vec<w::GraphicsPipelineDesc>,
    ) -> Vec<Result<B::GraphicsPipeline, hal::pso::CreationError>> {
        let gpu = &mut self.rehub.gpus.lock().unwrap()[gpu_id];
        let shader_store = self.rehub.shaders.read().unwrap();
        let rp_store = self.rehub.render_passes.read().unwrap();
        let layout_store = self.rehub.pipe_layouts.read().unwrap();

        let descs = descriptors
            .iter()
            .map(|desc| {
                let shaders = hal::pso::GraphicsShaderSet {
                    vertex: hal::pso::EntryPoint {
                        module: &shader_store[desc.shaders.vs.module_id],
                        entry: &desc.shaders.vs.name,
                    },
                    geometry: None,
                    hull: None,
                    domain: None,
                    fragment: desc.shaders.fs.as_ref().map(|s| hal::pso::EntryPoint {
                        module: &shader_store[s.module_id],
                        entry: &s.name,
                    }),
                };
                let layout = &layout_store[desc.layout_id];
                let subpass = hal::pass::Subpass {
                    index: desc.subpass as _,
                    main_pass: &rp_store[desc.renderpass_id],
                };
                (shaders, layout, subpass, &desc.inner)
            })
            .collect::<Vec<_>>();

        gpu.device.create_graphics_pipelines(&descs)
    }

    fn create_sampler(
        &mut self,
        gpu_id: w::GpuId,
        desc: hal::image::SamplerInfo,
    ) -> w::SamplerInfo {
        let device = &mut self.rehub.gpus.lock().unwrap()[gpu_id].device;

        let sampler = device.create_sampler(desc);

        w::SamplerInfo {
            id: self.rehub.samplers.write().unwrap().push(sampler),
        }
    }

    fn create_image_view(
        &mut self,
        gpu_id: w::GpuId,
        image_id: w::ImageId,
        format: hal::format::Format,
        range: hal::image::SubresourceRange,
    ) -> w::ImageViewInfo {
        let device = &mut self.rehub.gpus.lock().unwrap()[gpu_id].device;
        let image = &self.rehub.images.read().unwrap()[image_id];

        let view = device.create_image_view(
            image,
            format,
            hal::format::Swizzle::NO,
            range,
        ).unwrap();

        w::ImageViewInfo {
            id: self.rehub.image_views.write().unwrap().push(view),
        }
    }

    fn update_descriptor_sets(
        &mut self,
        gpu_id: w::GpuId,
        set_writes: Vec<w::DescriptorSetWrite>,
    ) {
        let device = &mut self.rehub.gpus.lock().unwrap()[gpu_id].device;
        let descriptor_store = self.rehub.descriptors.read().unwrap();
        let sampler_store = self.rehub.samplers.read().unwrap();
        let iv_store = self.rehub.image_views.read().unwrap();

        let mut writes = Vec::new();
        for w in set_writes {
            let write = match w.ty {
                hal::pso::DescriptorType::Sampler => {
                    let objects = w.descriptors
                        .into_iter()
                        .map(|(id, _)| &sampler_store[id])
                        .collect();
                    hal::pso::DescriptorWrite::Sampler(objects)
                }
                hal::pso::DescriptorType::SampledImage => {
                    let objects = w.descriptors
                        .into_iter()
                        .map(|(id, layout)| (&iv_store[id], layout))
                        .collect();
                    hal::pso::DescriptorWrite::SampledImage(objects)
                }
                _ => { unimplemented!() } //TODO
            };
            writes.push(hal::pso::DescriptorSetWrite {
                set: &descriptor_store[w.set],
                binding: w.binding,
                array_offset: w.array_offset,
                write,
            });
        }

        device.update_descriptor_sets(&writes);
    }
}
