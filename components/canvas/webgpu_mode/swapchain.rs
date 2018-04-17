use canvas_traits::hal;

use euclid::Size2D;


pub struct Swapchain<B: hal::Backend> {
    images: Vec<B::Image>,
}

impl<B: hal::Backend> Swapchain<B> {
    pub fn new(_device: &B::Device, _size: Size2D<u32>) -> Self {
        Swapchain {
            images: Vec::new()
        }
    }
}
