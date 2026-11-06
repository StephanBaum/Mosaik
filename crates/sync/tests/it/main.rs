//! Integration tests of `mosaic-sync`: the folder transport, the sync
//! engine over simulated file-sync clients, rotations, joining from a
//! folder and a hostile folder. Built as one binary.

mod chaos;
mod cloudsim;
mod common;
mod folder_transport;
mod join_folder;
mod malicious_folder;
mod rotation_folder;
mod sim_folder;
