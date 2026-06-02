// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![no_main]

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    find::ntfind_main();
}
