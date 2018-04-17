/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
//
// WebGPU IDL definitions scraped from the WebGPU sketch:
//

typedef boolean wg_bool;
typedef unsigned long wg_u32;


dictionary WebGPUDeviceDescriptor {
    required WebGPUExtensions extensions;
    required WebGPUFeatures features;
    required WebGPULimits limits;
    // TODO are other things configurable like queues?
};

// WebGPU "namespace" used for device creation
[Exposed=Window]
interface WebGPU {
    [NewObject] static WebGPU instance();

    WebGPUDevice createDevice(WebGPUDeviceDescriptor descriptor);
};
