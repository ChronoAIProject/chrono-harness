//! 本宿主的源码构建、共享启动和所有者升级；政策仍来自宿主登记。
#![allow(unexpected_cfgs)]

pub mod bindings;
pub mod construction;
pub mod environment;
pub mod execution;
pub mod profile;
pub mod source;
pub mod verified_bindings;
