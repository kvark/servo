/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![crate_name = "webmetal"]
#![crate_type = "rlib"]
#![feature(plugin)]
#![feature(proc_macro)]
#![plugin(plugins)]

#[macro_use] extern crate serde_derive;
extern crate shared_library;
extern crate vk_sys as vk;

use shared_library::dynamic_library::DynamicLibrary;
use std::{iter, mem, ptr};
use std::ffi::{CStr, CString};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct WebMetalCapabilities;

struct PhysicalDeviceInfo {
    device: vk::PhysicalDevice,
    _properties: vk::PhysicalDeviceProperties,
    queue_families: Vec<vk::QueueFamilyProperties>,
    memory: vk::PhysicalDeviceMemoryProperties,
    _features: vk::PhysicalDeviceFeatures,
}

impl PhysicalDeviceInfo {
    #[allow(unsafe_code)]
    pub fn new(dev: vk::PhysicalDevice, vk: &vk::InstancePointers) -> PhysicalDeviceInfo {
        PhysicalDeviceInfo {
            device: dev,
            _properties: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceProperties(dev, &mut out);
                out
            },
            queue_families: unsafe {
                let mut num = 0;
                vk.GetPhysicalDeviceQueueFamilyProperties(dev, &mut num, ptr::null_mut());
                let mut families = Vec::with_capacity(num as usize);
                vk.GetPhysicalDeviceQueueFamilyProperties(dev, &mut num, families.as_mut_ptr());
                families.set_len(num as usize);
                families
            },
            memory: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceMemoryProperties(dev, &mut out);
                out
            },
            _features: unsafe {
                let mut out = mem::zeroed();
                vk.GetPhysicalDeviceFeatures(dev, &mut out);
                out
            },
        }
    }
}


const LAYERS: &'static [&'static str] = &[
];
const LAYERS_DEBUG: &'static [&'static str] = &[
    "VK_LAYER_LUNARG_standard_validation",
];
const EXTENSIONS: &'static [&'static str] = &[
    "VK_KHR_surface",
];
const EXTENSIONS_DEBUG: &'static [&'static str] = &[
    "VK_KHR_surface",
    "VK_EXT_debug_report",
];
const DEV_EXTENSIONS: &'static [&'static str] = &[
    "VK_KHR_swapchain",
];
const SURFACE_EXTENSIONS: &'static [&'static str] = &[
    // Platform-specific WSI extensions
    "VK_KHR_xlib_surface",
    "VK_KHR_xcb_surface",
    "VK_KHR_wayland_surface",
    "VK_KHR_mir_surface",
    "VK_KHR_android_surface",
    "VK_KHR_win32_surface",
];

pub struct SwapChain {
    _image: vk::Image,
    _memory: vk::DeviceMemory,
    _views: Vec<vk::ImageView>,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct CommandBuffer {
    inner: vk::CommandBuffer,
    family_index: u32,
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        //TODO
    }
}

pub struct Queue {
    inner: vk::Queue,
    family_index: u32,
    command_pool: vk::CommandPool,
}

pub struct Device {
    inner: vk::Device,
    pointers: vk::DevicePointers,
    _mem_system: u32,
    mem_video: u32,
}

impl Device {
    fn make_queue(&self, family_id: u32) -> Queue {
        let com_info = vk::CommandPoolCreateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            pNext: ptr::null(),
            flags: vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: family_id,
        };
        let mut com_pool = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.CreateCommandPool(self.inner, &com_info, ptr::null(), &mut com_pool)
        });

        let queue = unsafe {
            let mut out = mem::zeroed();
            self.pointers.GetDeviceQueue(self.inner, family_id, 0, &mut out);
            out
        };
        Queue {
            inner: queue,
            family_index: family_id,
            command_pool: com_pool,
        }
    }

    pub fn make_command_buffer(&self, queue: &Queue) -> CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            pNext: ptr::null(),
            commandPool: queue.command_pool,
            level: vk::COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
        };
        let begin_info = vk::CommandBufferBeginInfo {
            sType: vk::STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            pNext: ptr::null(),
            flags: 0,
            pInheritanceInfo: ptr::null(),
        };

        let mut buf = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.AllocateCommandBuffers(self.inner, &alloc_info, &mut buf)
        });
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.BeginCommandBuffer(buf, &begin_info)
        });

        CommandBuffer {
            inner: buf,
            family_index: queue.family_index,
        }
    }

    pub fn execute(&self, queue: &Queue, com: CommandBuffer) {
        assert_eq!(queue.family_index, com.family_index);
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.EndCommandBuffer(com.inner)
        });
        let submit_info = vk::SubmitInfo {
            sType: vk::STRUCTURE_TYPE_SUBMIT_INFO,
            commandBufferCount: 1,
            pCommandBuffers: &com.inner,
            .. unsafe { mem::zeroed() }
        };
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.QueueSubmit(queue.inner, 1, &submit_info, 0)
        });
    }
}


#[derive(Debug)]
pub struct InitError;

impl Device {
    pub fn new(debug: bool)
               -> Result<(Device, Queue, WebMetalCapabilities), InitError> {
        let (layers, extensions) = if debug {
            (LAYERS_DEBUG, EXTENSIONS_DEBUG)
        } else {
            (LAYERS, EXTENSIONS)
        };
        let dev_extensions = DEV_EXTENSIONS;

        let lib_name = if cfg!(target_os = "windows") {
            "vulkan-1.dll"
        } else {
            "libvulkan.so.1"
        };
        let dynamic_lib = DynamicLibrary::open(Some(Path::new(lib_name)))
                                         .expect("Unable to open vulkan shared library");

        let lib = vk::Static::load(|name| unsafe {
            let name = name.to_str().unwrap();
            dynamic_lib.symbol(name).unwrap()
        });
        let entry_points = vk::EntryPoints::load(|name| unsafe {
            mem::transmute(lib.GetInstanceProcAddr(0, name.as_ptr()))
        });

        let app_info = vk::ApplicationInfo {
            sType: vk::STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: ptr::null(),
            pApplicationName: "servo".as_ptr() as *const _,
            applicationVersion: 1,
            pEngineName: "webmetal".as_ptr() as *const _,
            engineVersion: 0x1000, //TODO
            apiVersion: 0x400000, //TODO
        };

        let instance_extensions = {
            let mut num = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.EnumerateInstanceExtensionProperties(ptr::null(), &mut num, ptr::null_mut())
            });
            let mut out = Vec::with_capacity(num as usize);
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.EnumerateInstanceExtensionProperties(ptr::null(), &mut num, out.as_mut_ptr())
            });
            unsafe { out.set_len(num as usize); }
            out
        };

        // Check our surface extensions against the available extensions
        let surface_extensions = SURFACE_EXTENSIONS.iter().filter_map(|ext| {
            instance_extensions.iter().find(|inst_ext| {
                unsafe { CStr::from_ptr(inst_ext.extensionName.as_ptr()) == CStr::from_ptr(ext.as_ptr() as *const i8) }
            }).and_then(|_| Some(*ext))
        }).collect::<Vec<&str>>();

        let instance = {
            let cstrings = layers.iter().chain(extensions.iter())
                                        .chain(surface_extensions.iter())
                             .map(|&s| CString::new(s).unwrap())
                             .collect::<Vec<_>>();
            let str_pointers = cstrings.iter()
                                       .map(|s| s.as_ptr())
                                       .collect::<Vec<_>>();

            let create_info = vk::InstanceCreateInfo {
                sType: vk::STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                pApplicationInfo: &app_info,
                enabledLayerCount: layers.len() as u32,
                ppEnabledLayerNames: str_pointers.as_ptr(),
                enabledExtensionCount: (extensions.len() + surface_extensions.len()) as u32,
                ppEnabledExtensionNames: str_pointers[layers.len()..].as_ptr(),
            };
            let mut out = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                entry_points.CreateInstance(&create_info, ptr::null(), &mut out)
            });
            out
        };

        let inst_pointers = vk::InstancePointers::load(|name| unsafe {
            mem::transmute(lib.GetInstanceProcAddr(instance, name.as_ptr()))
        });

        let physical_devices = {
            let mut num = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.EnumeratePhysicalDevices(instance, &mut num, ptr::null_mut())
            });
            let mut devices = Vec::with_capacity(num as usize);
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.EnumeratePhysicalDevices(instance, &mut num, devices.as_mut_ptr())
            });
            unsafe { devices.set_len(num as usize); }
            devices
        };

        let phys_infos = physical_devices.iter()
            .map(|dev| PhysicalDeviceInfo::new(*dev, &inst_pointers))
            .collect::<Vec<_>>();

        let (dev, (qf_id, _))  = phys_infos.iter()
            .flat_map(|d| iter::repeat(d).zip(d.queue_families.iter().enumerate()))
            .find(|&(_, (_, qf))| qf.queueFlags & vk::QUEUE_GRAPHICS_BIT != 0)
            .unwrap();
        //info!("Chosen physical device {:?} with queue family {}", dev.device, qf_id);

        let mvid_id = dev.memory.memoryTypes.iter().take(dev.memory.memoryTypeCount as usize)
                                .position(|mt| (mt.propertyFlags & vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT != 0)
                                            && (mt.propertyFlags & vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT != 0))
                                .unwrap() as u32;
        let msys_id = dev.memory.memoryTypes.iter().take(dev.memory.memoryTypeCount as usize)
                                .position(|mt| mt.propertyFlags & vk::MEMORY_PROPERTY_HOST_COHERENT_BIT != 0)
                                .unwrap() as u32;

        let vk_device = {
            let cstrings = dev_extensions.iter()
                                         .map(|&s| CString::new(s).unwrap())
                                         .collect::<Vec<_>>();
            let str_pointers = cstrings.iter().map(|s| s.as_ptr())
                                       .collect::<Vec<_>>();

            let queue_info = vk::DeviceQueueCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueFamilyIndex: qf_id as u32,
                queueCount: 1,
                pQueuePriorities: &1.0,
            };
            let features = unsafe{ mem::zeroed() };

            let dev_info = vk::DeviceCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueCreateInfoCount: 1,
                pQueueCreateInfos: &queue_info,
                enabledLayerCount: 0,
                ppEnabledLayerNames: ptr::null(),
                enabledExtensionCount: str_pointers.len() as u32,
                ppEnabledExtensionNames: str_pointers.as_ptr(),
                pEnabledFeatures: &features,
            };
            let mut out = 0;
            assert_eq!(vk::SUCCESS, unsafe {
                inst_pointers.CreateDevice(dev.device, &dev_info, ptr::null(), &mut out)
            });
            out
        };

        let dev_pointers = vk::DevicePointers::load(|name| unsafe {
            inst_pointers.GetDeviceProcAddr(vk_device, name.as_ptr()) as *const _
        });

        let device = Device {
            inner: vk_device,
            pointers: dev_pointers,
            _mem_system: msys_id,
            mem_video: mvid_id,
        };
        let queue = device.make_queue(qf_id as u32);

        Ok((device, queue, WebMetalCapabilities))
    }

    fn alloc(&self, mem_id: u32, reqs: vk::MemoryRequirements) -> vk::DeviceMemory {
        let info = vk::MemoryAllocateInfo {
            sType: vk::STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            pNext: ptr::null(),
            allocationSize: reqs.size,
            memoryTypeIndex: mem_id,
        };
        let mut mem = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.AllocateMemory(self.inner, &info, ptr::null(), &mut mem)
        });
        mem
    }

    pub fn create_swap_chain(&self, width: u32, height: u32, count: u32) -> SwapChain {
        let image_info = vk::ImageCreateInfo {
            sType: vk::STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            imageType: vk::IMAGE_TYPE_2D,
            format: vk::FORMAT_R8G8B8A8_SRGB,
            extent: vk::Extent3D {
                width: width,
                height: height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: count,
            samples: vk::SAMPLE_COUNT_1_BIT,
            tiling: vk::IMAGE_TILING_OPTIMAL,
            usage: vk::IMAGE_USAGE_TRANSFER_SRC_BIT | vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            sharingMode: vk::SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: ptr::null(),
            initialLayout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };

        let mut image = 0;
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.CreateImage(self.inner, &image_info, ptr::null(), &mut image)
        });
        let reqs = unsafe {
            let mut out = mem::zeroed();
            self.pointers.GetImageMemoryRequirements(self.inner, image, &mut out);
            out
        };
        let memory = self.alloc(self.mem_video, reqs);
        assert_eq!(vk::SUCCESS, unsafe {
            self.pointers.BindImageMemory(self.inner, image, memory, 0)
        });

        SwapChain {
            _image: image,
            _memory: memory,
            _views: Vec::new(),
        }
    }
}
