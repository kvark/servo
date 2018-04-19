use canvas_traits::hal::{self, Device};
use self::hal::command::RawCommandBuffer;
use self::hal::pool::RawCommandPool;

use euclid::Size2D;


pub struct Frame<B: hal::Backend> {
    pub image: B::Image,
    pub fence: B::Fence,
    pub command_buffer: B::CommandBuffer,
}

pub struct Swapchain<B: hal::Backend> {
    frames: Vec<Frame<B>>,
    id_to_present: usize,
    id_to_acquire: usize,
    _command_pool: B::CommandPool,
}

fn align(address: u64, alignment: u64) -> u64 {
    (address + alignment - 1) & !(alignment - 1)
}

impl<B: hal::Backend> Swapchain<B> {
    pub fn new(
        size: Size2D<u32>, num_frames: usize, format: hal::format::Format,
        device: &B::Device, queue_family: hal::queue::QueueFamilyId,
        memory_types: &[hal::MemoryType]
    ) -> Self {
        use self::hal::image as i;

        let mut total_size = 0;
        let mut type_mask = !0u64;
        let mut preframes = Vec::new();

        for _ in 0 .. num_frames {
            let unbound = device
                .create_image(
                    i::Kind::D2(size.width, size.height, 1, 1),
                    1,
                    format,
                    i::Tiling::Optimal,
                    i::Usage::TRANSFER_SRC | i::Usage::COLOR_ATTACHMENT,
                    i::StorageFlags::empty(),
                )
                .unwrap();
            let req = device.get_image_requirements(&unbound);
            type_mask &= req.type_mask;
            let offset = align(total_size, req.alignment);
            total_size = offset + req.size;
            preframes.push((unbound, offset));
        }

        let selected_type = memory_types
            .iter()
            .enumerate()
            .position(|(i, mt)|
                ((1 << i) as u64 & type_mask != 0) &&
                mt.properties.contains(hal::memory::Properties::DEVICE_LOCAL)
            )
            .unwrap()
            .into();
        let memory = device
            .allocate_memory(selected_type, total_size)
            .unwrap();

        let mut command_pool = device
            .create_command_pool(queue_family, hal::pool::CommandPoolCreateFlags::RESET_INDIVIDUAL);
        let cmd_buffers = command_pool.allocate(num_frames, hal::command::RawLevel::Primary);

        let frames = preframes
            .into_iter()
            .zip(cmd_buffers)
            .map(|((unbound, offset), command_buffer)| {
                let image = device
                    .bind_image_memory(&memory, offset, unbound)
                    .unwrap();
                let fence = device.create_fence(true);
                Frame {
                    image,
                    fence,
                    command_buffer,
                }
            })
            .collect();

        Swapchain {
            frames,
            id_to_present: 0,
            id_to_acquire: 0,
            _command_pool: command_pool,
        }
    }

    pub fn acquire_frame(&mut self, device: &B::Device) -> usize {
        let id = self.id_to_acquire;
        self.id_to_acquire = if id + 1 >= self.frames.len() { 0 } else { id + 1 };
        device.wait_for_fence(&self.frames[id].fence, !0);
        assert_ne!(self.id_to_acquire, self.id_to_present, "Swapchain frame capacity exceeded!");
        id
    }

    #[allow(unsafe_code)]
    pub fn present(&mut self) -> (hal::command::Submit<B, hal::Graphics, hal::command::OneShot, hal::command::Primary>, &B::Fence) {
        assert_ne!(self.id_to_present, self.id_to_acquire, "Swapchain frame capacity exceeded!");
        let id = self.id_to_present;
        self.id_to_present = if id + 1 >= self.frames.len() { 0 } else { id + 1 };
        let frame = &mut self.frames[id];
        frame.command_buffer.begin(
            hal::command::CommandBufferFlags::ONE_TIME_SUBMIT,
            hal::command::CommandBufferInheritanceInfo::default(),
        );
        let cmd_buf = unsafe {
            hal::command::CommandBuffer::new(&mut frame.command_buffer)
        };
        //TODO: present
        (cmd_buf.finish(), &frame.fence)
    }
}
