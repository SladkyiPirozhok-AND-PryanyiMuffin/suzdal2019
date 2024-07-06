use std::fs;
use std::error::Error;
use std::path::PathBuf;

use tauri::{ AppHandle, Manager, State };
use tauri::async_runtime::spawn;

use rfd::{ FileDialog, MessageButtons, MessageDialog, MessageDialogResult };

use crate::{ DataState, ImageState, BinType, show_spinner, hide_spinner, show_error_message, update_window_title };
use crate::data_view::DataView;
use crate::data_pack::save_data_pack;
use crate::sprite_pack::get_image_data ;
use crate::smacard::read_card;
use crate::firmware::{ read_firmware, save_menu_strings };

#[tauri::command]
pub fn open_bin(handle: AppHandle) {
	if continue_if_modified(&handle) {
		spawn(async move {
			let data_state: State<DataState> = handle.state();
			let image_state: State<ImageState> = handle.state();

			let mut file_dialog = FileDialog::new()
				.add_filter("firmware dump", &["bin"]);
			if let Some(base_path) = data_state.base_path.lock().unwrap().as_ref() {
				file_dialog = file_dialog.set_directory(base_path);
			}

			if let Some(path) = file_dialog.pick_file() {
				show_spinner(&handle);

				let raw_data = fs::read(&path).unwrap();
				*data_state.original_data.lock().unwrap() = Some(raw_data.clone());
				let data = DataView::new(&raw_data);

				let bin_type = if &raw_data[0..14] == b"GP-SPIF-HEADER" {
					BinType::Firmware
				} else {
					BinType::SmaCard
				};

				match &bin_type {
					BinType::SmaCard => {
						match read_card(&data) {
							Ok(card) => {
								*data_state.data_pack.lock().unwrap() = Some(card.data_pack.clone());
								*data_state.sprite_pack.lock().unwrap() = Some(card.sprite_pack.clone());
								if let Ok(image_data) = get_image_data(&card.sprite_pack.clone()) {
									*image_state.images.lock().unwrap() = image_data;
								}
								handle.emit("show_card", card).unwrap();
							},
							Err(why) => show_error_message(why)
						}
					},

					BinType::Firmware => {
						match read_firmware(&data) {
							Ok(firmware) => {
								*data_state.data_pack.lock().unwrap() = Some(firmware.data_pack.clone());
								*data_state.sprite_pack.lock().unwrap() = Some(firmware.sprite_pack.clone());
								if let Ok(image_data) = get_image_data(&firmware.sprite_pack.clone()) {
									*image_state.images.lock().unwrap() = image_data;
								}
								*data_state.menu_strings.lock().unwrap() = Some(firmware.menu_strings.clone());
								handle.emit("show_firmware", firmware).unwrap();
							},
							Err(why) => show_error_message(why)
						}
					}
				}

				*data_state.is_modified.lock().unwrap() = false;
				*data_state.bin_type.lock().unwrap() = Some(bin_type);
				*data_state.file_path.lock().unwrap() = Some(path.to_path_buf());
				*data_state.base_path.lock().unwrap() = Some(path.parent().unwrap().to_path_buf());

				hide_spinner(&handle);

				update_window_title(&handle);
			}
		});
	}
}

#[tauri::command]
pub fn save_bin(handle: AppHandle) {
	let data_state: State<DataState> = handle.state();
	let no_data = if let None = *data_state.data_pack.lock().unwrap() { true } else { false };
	if no_data {
		show_error_message("No data to save".into());

	} else {
		let file_path_opt = data_state.file_path.lock().unwrap().clone();
		match file_path_opt {
			Some(file_path) => {
				if let Err(why) = save(&handle, &file_path) {
					show_error_message(why);
				}
			},
			None => save_bin_as(handle)
		}
	}
}

#[tauri::command]
pub fn save_bin_as(handle: AppHandle) {
	spawn(async move {
		let data_state: State<DataState> = handle.state();
		let no_data = if let None = *data_state.data_pack.lock().unwrap() { true } else { false };
		if no_data {
			show_error_message("No data to save".into());

		} else {
			let mut file_dialog = FileDialog::new()
				.add_filter("firmware dump", &["bin"]);
			if let Some(base_path) = data_state.base_path.lock().unwrap().as_ref() {
				file_dialog = file_dialog.set_directory(base_path);
			}

			if let Some(path) = file_dialog.save_file() {
				show_spinner(&handle);
				match save(&handle, &path) {
					Ok(()) => {
						*data_state.file_path.lock().unwrap() = Some(path.to_path_buf());
						*data_state.base_path.lock().unwrap() = Some(path.parent().unwrap().to_path_buf());
					},
					Err(why) => show_error_message(why)
				}
				hide_spinner(&handle);
			}
		}
	});
}

pub fn save(handle: &AppHandle, path: &PathBuf) -> Result<(), Box<dyn Error>> {
	let data_state: State<DataState> = handle.state();
	// let image_state: State<ImageState> = handle.state();

	if let Some(bin_type) = data_state.bin_type.lock().unwrap().as_ref() {
		match bin_type {
			BinType::Firmware => {
				match data_state.original_data.lock().unwrap().as_ref() {
					Some(original_data) => {
						match data_state.data_pack.lock().unwrap().as_ref() {
							Some(data_pack) => {
								match data_state.menu_strings.lock().unwrap().as_ref() {
									Some(menu_strings) => {
										let new_data = save_data_pack(&original_data, &data_pack)?;
										let new_data = save_menu_strings(&new_data, &menu_strings)?;
										if original_data.len() == new_data.len() {
											fs::write(path, &new_data)?;
											*data_state.is_modified.lock().unwrap() = false;
											update_window_title(&handle);
										} else {
											return Err(format!("New data is {} bytes, but original is {} bytes", new_data.len(), original_data.len()).into());
										}
									},
									None => return Err("No menu strings found in current file".into())
								}
							},
							None => return Err("No data pack found in current file".into())
						}
					},
					None => return Err("No original data found for current file".into())
				}
			},
			_ => return Err("Can only save firmware currently".into())
		}
	}

	Ok(())
}

pub fn continue_if_modified(handle: &AppHandle) -> bool {
	let data_state: State<DataState> = handle.state();
	if *data_state.is_modified.lock().unwrap() {
		let dialog_result = MessageDialog::new()
			.set_title("File modified")
			.set_description("Do you want to continue anyway and lose any unsaved work?")
			.set_buttons(MessageButtons::YesNo)
			.show();
		match dialog_result {
			MessageDialogResult::Yes => true,
			_ => false
		}
	} else {
		true
	}
}
