/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuQueueFlags;
typedef unsigned short WebGpuQueueCount;
typedef unsigned long WebGpuQueueFamilyId;

dictionary WebGpuQueueFamilyInfo {
	required WebGpuQueueFlags flags;
	required WebGpuQueueCount count;
	required WebGpuQueueFamilyId id;
};

dictionary WebGpuRequestedQueues {
	required WebGpuQueueFamilyId id;
	required WebGpuQueueCount count;
};

interface WebGpuAdapter {
	const WebGpuQueueFlags QUEUE_GENERAL = 0x1;
	const WebGpuQueueFlags QUEUE_COMPUTE  = 0x2;
	const WebGpuQueueFlags QUEUE_TRANSFER = 0x4;

    sequence<WebGpuQueueFamilyInfo> getQueueFamilies();
    void open(sequence<WebGpuRequestedQueues> queues);
};
