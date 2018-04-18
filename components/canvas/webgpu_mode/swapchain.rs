use canvas_traits::hal::{self, Device};

use euclid::Size2D;


pub struct Swapchain<B: hal::Backend> {
    frames: Vec<(B::Image, B::Fence)>,
    id_next: usize,
}

fn align(address: u64, alignment: u64) -> u64 {
    (address + alignment - 1) & !(alignment - 1)
}

impl<B: hal::Backend> Swapchain<B> {
    pub fn new(
        size: Size2D<u32>, num_frames: usize, format: hal::format::Format,
        device: &B::Device, memory_types: &[hal::MemoryType]
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

        let frames = preframes
            .into_iter()
            .map(|(unbound, offset)| {
                let image = device
                    .bind_image_memory(&memory, offset, unbound)
                    .unwrap();
                let fence = device.create_fence(true);
                (image, fence)
            })
            .collect();

        Swapchain {
            frames,
            id_next: 0,
        }
    }

    pub fn acquire_frame(&mut self, device: &B::Device) -> usize {
        let id = self.id_next;
        self.id_next = if id + 1 >= self.frames.len() { 0 } else { id + 1 };
        device.wait_for_fence(&self.frames[id].1, !0);
        id
    }
}
