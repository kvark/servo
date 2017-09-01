/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use webgpu::backend;
use webgpu::gpu::{self,
    Adapter, Device, QueueFamily,
    RawCommandBuffer, RawCommandPool, RawCommandQueue,
};

use euclid::Size2D;

use std::thread;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{mpsc, Arc};

use webgpu_mode::{LazyVec, ResourceHub};
/// WebGL Threading API entry point that lives in the constellation.
/// It allows to get a WebGpuThread handle for each script pipeline.
pub use ::webgpu_mode::WebGpuThreads;


enum PoolCommand<B: gpu::Backend> {
    FinishBuffer(w::CommandBufferId, w::SubmitEpoch, B::CommandBuffer),
    Reset,
    Destroy,
}

struct CommandPoolHandle<B: gpu::Backend> {
    _join: thread::JoinHandle<()>,
    //Note: you can't have more than one buffer encoded at a single time,
    // but you can have multiple finished command buffers ready for submission.
    submits: HashMap<w::CommandBufferId, (w::SubmitEpoch, B::CommandBuffer)>,
    receiver: mpsc::Receiver<PoolCommand<B>>,
    is_alive: bool,
}

impl<B: gpu::Backend> CommandPoolHandle<B> {
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
        while let Ok(command) = self.receiver.recv() {
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

pub struct WebGpuThread<B: gpu::Backend> {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
    adapters: Vec<B::Adapter>,
    gpus: LazyVec<gpu::Gpu<B>>,
    heaps: LazyVec<B::Heap>,
    rehub: Arc<ResourceHub<B>>,
    command_pools: LazyVec<CommandPoolHandle<B>>,
}

impl WebGpuThread<backend::Backend> {
    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start() -> w::WebGpuSender<w::WebGpuMsg> {
        let (sender, receiver) = w::webgpu_channel::<w::WebGpuMsg>().unwrap();
        let result = sender.clone();
        let enable_debug = true; //TODO
        thread::Builder::new().name("WebGpuThread".to_owned()).spawn(move || {
            let instance = backend::Instance::create("Servo", 1, enable_debug);
            let mut renderer: Self = WebGpuThread {
                next_webgpu_id: 0,
                adapters: instance.enumerate_adapters(),
                gpus: LazyVec::new(),
                heaps: LazyVec::new(),
                rehub: ResourceHub::new(),
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

        result
    }
}

impl<B: gpu::Backend> WebGpuThread<B> {
    /// Handles a generic WebGpuMsg message
    fn handle_msg(&mut self, msg: w::WebGpuMsg, webgpu_chan: &w::WebGpuChan) -> bool {
        match msg {
            w::WebGpuMsg::CreateContext(sender) => {
                let init = self
                    .create_webgpu_context()
                    .map(|(id, adapters)| w::ContextInfo {
                        sender: w::WebGpuMsgSender::new(id, webgpu_chan.clone()),
                        adapters,
                    });
                sender.send(init).unwrap();
            }
            w::WebGpuMsg::OpenAdapter { adapter_id, queue_families, result } => {
                let adapter = &mut self.adapters[adapter_id as usize];
                let all_families = adapter.get_queue_families();
                let families = queue_families
                    .iter()
                    .map(|&(id, count)| {
                        let (ref family, type_) = all_families[id as usize];
                        (family, type_, count as u32)
                    })
                    .collect::<Vec<_>>();
                let gpu = adapter.open(&families);
                let general_queues = (0 .. gpu.general_queues.len() as w::QueueId).collect();
                let info = w::GpuInfo {
                    id: self.gpus.push(gpu),
                    general: general_queues,
                };
                result.send(info).unwrap();
            }
            w::WebGpuMsg::BuildSwapchain { gpu_id, format, size, result } => {
                let swapchain = self.build_swapchain(gpu_id, format, size);
                result.send(swapchain).unwrap();
            }
            w::WebGpuMsg::CreateCommandPool { gpu_id, queue_id, result } => {
                let command_pool = self.create_command_pool(gpu_id, queue_id);
                result.send(command_pool).unwrap();
            }
            w::WebGpuMsg::Submit { gpu_id, queue_id, command_buffers, .. } => {
                let cmd_buffers = command_buffers
                    .into_iter()
                    .map(|info| {
                        self.command_pools[info.pool_id]
                            .extract_submit(info.cb_id, info.submit_epoch)
                    })
                    .collect();
                self.submit(gpu_id, queue_id, cmd_buffers);
            }
            w::WebGpuMsg::Present(image_id) => {
                //TODO
            }
            w::WebGpuMsg::Exit => {
                return true;
            }
            w::WebGpuMsg::CreateFence { gpu_id, set, result } => {
                let fence = self.create_fence(gpu_id, set);
                result.send(fence).unwrap();
            }
            w::WebGpuMsg::ResetFences { gpu_id, fences } => {
                let gpu = &mut self.gpus[gpu_id];
                let store = self.rehub.fences.read().unwrap();
                let fences_raw = fences
                    .into_iter()
                    .map(|f| &store[f])
                    .collect::<Vec<_>>();
                gpu.device.reset_fences(&fences_raw);
            }
            w::WebGpuMsg::WaitForFences { gpu_id, fences, mode, timeout } => {
                let gpu = &mut self.gpus[gpu_id];
                let store = self.rehub.fences.read().unwrap();
                let fences_raw = fences
                    .into_iter()
                    .map(|f| &store[f])
                    .collect::<Vec<_>>();
                gpu.device.wait_for_fences(&fences_raw, mode, timeout);
            }
            w::WebGpuMsg::CreateFramebuffer { gpu_id, desc, result } => {
                let framebuffer = self.create_framebuffer(gpu_id, desc);
                result.send(framebuffer).unwrap();
            }
            w::WebGpuMsg::CreateRenderpass { gpu_id, desc, result } => {
                let renderpass = self.create_renderpass(gpu_id, desc);
                result.send(renderpass).unwrap();
            }
            w::WebGpuMsg::ViewImageAsRenderTarget { gpu_id, image_id, format, result } => {
                let rtv = self.view_image_as_render_target(gpu_id, image_id, format);
                result.send(rtv).unwrap();
            }
        }

        false
    }

    /// Creates a new WebGpuContext
    fn create_webgpu_context(&mut self
    ) -> Result<(w::WebGpuContextId, Vec<w::AdapterInfo>), String>
    {
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
                            count: family.num_queues() as u8,
                            original_id: qid as w::QueueFamilyId,
                        }
                    })
                    .collect();
                w::AdapterInfo {
                    info: ad.get_info().clone(),
                    queue_families,
                    original_id: aid as w::AdapterId,
                }
            })
            .collect();
        let id = w::WebGpuContextId(self.next_webgpu_id);
        self.next_webgpu_id += 1;
        Ok((id, adapters))
    }

    fn build_swapchain(&mut self,
        gpu_id: w::GpuId,
        format: gpu::format::Format,
        size: Size2D<u32>,
    ) -> w::SwapchainInfo {
        let gpu = &mut self.gpus[gpu_id];
        let mut image_store = self.rehub.images.write().unwrap();
        let num_frames = 3; //TODO
        let alignment = 0x3F;
        let bytes_per_image = 4 *
            (size.width as u64 | alignment + 1)*
            (size.height as u64| alignment + 1);

        let heap = {
            let heap_type = gpu.heap_types
                .iter()
                .find(|ht| ht.properties.contains(gpu::memory::DEVICE_LOCAL))
                .unwrap();
            gpu.device.create_heap(
                heap_type,
                gpu::device::ResourceHeapType::Images,
                num_frames * bytes_per_image,
            ).unwrap()
        };

        let images = (0 .. num_frames).map(|i| {
            let unbound_image = gpu.device.create_image(
                gpu::image::Kind::D2(
                    size.width as gpu::image::Size,
                    size.height as gpu::image::Size,
                    gpu::image::AaMode::Single,
                ),
                1,
                format,
                gpu::image::COLOR_ATTACHMENT | gpu::image::TRANSFER_SRC,
            ).unwrap();
            let image = gpu.device.bind_image_memory(
                &heap,
                i * bytes_per_image,
                unbound_image,
            ).unwrap();
            image_store.push(image)
        }).collect();

        w::SwapchainInfo {
            heap_id: self.heaps.push(heap),
            images,
        }
    }

    #[allow(unsafe_code)]
    fn create_command_pool(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
    ) -> w::CommandPoolInfo
    {
        let gpu = &mut self.gpus[gpu_id];
        let queue = gpu.general_queues[queue_id as usize].as_raw();//TODO
        let pool = unsafe {
            B::CommandPool::from_queue(queue)
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

                    for cb in cbufs.into_iter() {
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
                    let id = active_id.unwrap();
                    com_buffers[id].finish();
                    let submit = com_buffers[id].clone();
                    let cmd = PoolCommand::FinishBuffer(id, submit_epoch, submit);
                    active_id = None;
                    channel.send(cmd).unwrap();
                }
                w::WebGpuCommand::PipelineBarrier(buffer_bars, image_bars) => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let buffer_store = rehub.buffers.read().unwrap();
                    let image_store = rehub.images.read().unwrap();

                    let buffer_iter = buffer_bars
                        .into_iter()
                        .map(|bar| gpu::memory::Barrier::Buffer {
                            state_src: bar.state_src,
                            state_dst: bar.state_dst,
                            target: &buffer_store[bar.target],
                            range: 0..1, //TODO
                        });

                    let image_iter = image_bars
                        .into_iter()
                        .map(|bar| gpu::memory::Barrier::Image {
                            state_src: bar.state_src,
                            state_dst: bar.state_dst,
                            target: &image_store[bar.target],
                            range: (0..1, 0..1), //TODO
                        });

                    let barriers = buffer_iter
                        .chain(image_iter)
                        .collect::<Vec<_>>();
                    cb.pipeline_barrier(&barriers);
                }
                w::WebGpuCommand::BeginRenderpass { renderpass, framebuffer, area, clear_values } => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    let pass = &rehub.renderpasses.read().unwrap()[renderpass];
                    let fbo = &rehub.framebuffers.read().unwrap()[framebuffer];
                    cb.begin_renderpass(pass, fbo, area, &clear_values, gpu::command::SubpassContents::Inline);
                }
                w::WebGpuCommand::EndRenderpass => {
                    let cb = &mut com_buffers[active_id.unwrap()];
                    cb.end_renderpass();
                }
            }
        }
    }

    #[allow(unsafe_code)]
    fn submit(&mut self,
        gpu_id: w::GpuId,
        queue_id: w::QueueId,
        cmd_buffers: Vec<B::CommandBuffer>,
    ) {
        let gpu = &mut self.gpus[gpu_id];
        let queue = gpu.general_queues[queue_id as usize].as_mut();
        let submission = gpu::RawSubmission {
            cmd_buffers: &cmd_buffers,
            wait_semaphores: &[],
            signal_semaphores: &[],
        };
        unsafe {
            queue.submit_raw(submission, None)
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
        let gpu = &mut self.gpus[gpu_id];

        let fence = gpu.device.create_fence(set);
        self.rehub.fences.write().unwrap().push(fence)
    }

    fn create_framebuffer(&mut self,
        gpu_id: w::GpuId,
        desc: w::FramebufferDesc,
    ) -> w::FramebufferInfo {
        let gpu = &mut self.gpus[gpu_id];

        let renderpass = &self.rehub.renderpasses.read().unwrap()[desc.renderpass];
        let rtv_store = self.rehub.rtvs.read().unwrap();
        let color_attachments = desc.colors
            .into_iter()
            .map(|id| &rtv_store[id])
            .collect::<Vec<_>>();
        let dsv_store = self.rehub.dsvs.read().unwrap();
        let depth_stencil_attachments = desc.depth_stencil
            .into_iter()
            .map(|id| &dsv_store[id])
            .collect::<Vec<_>>();
        let fbo = gpu.device.create_framebuffer(
            renderpass,
            &color_attachments,
            &depth_stencil_attachments,
            desc.width,
            desc.height,
            desc.layers,
        );

        w::FramebufferInfo {
            id: self.rehub.framebuffers.write().unwrap().push(fbo),
        }
    }

    fn create_renderpass(&mut self,
        gpu_id: w::GpuId,
        desc: w::RenderpassDesc,
    ) -> w::RenderpassInfo {
        let gpu = &mut self.gpus[gpu_id];

        let subpasses = desc.subpasses
            .iter()
            .map(|sp| gpu::pass::SubpassDesc {
                color_attachments: &sp.colors,
            })
            .collect::<Vec<_>>();
        let rp = gpu.device.create_renderpass(
            &desc.attachments,
            &subpasses,
            &desc.dependencies,
        );

        w::RenderpassInfo {
            id: self.rehub.renderpasses.write().unwrap().push(rp),
        }
    }

    fn view_image_as_render_target(&mut self,
        gpu_id: w::GpuId,
        image_id: w::ImageId,
        format: gpu::format::Format,
    ) -> w::RenderTargetViewInfo {
        let gpu = &mut self.gpus[gpu_id];
        let image = &self.rehub.images.read().unwrap()[image_id];
        let range = ((0..1), (0..1));

        let view = gpu.device.view_image_as_render_target(image, format, range).unwrap();

        w::RenderTargetViewInfo {
            id: self.rehub.rtvs.write().unwrap().push(view),
        }
    }
}
