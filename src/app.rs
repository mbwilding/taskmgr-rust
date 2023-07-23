use egui::Context;
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use sysinfo::{
    CpuExt, CpuRefreshKind, Process, ProcessExt, ProcessRefreshKind, RefreshKind, System,
    SystemExt, UserExt,
};

#[derive(Serialize, Deserialize, PartialEq)]
enum EWindow {
    Processes,
    Performance,
    AppHistory,
    StartupApps,
    Users,
    Details,
    Services,
    Settings,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum EProcessesSort {
    Name,
    User,
    Cpu,
    Memory,
    Disk,
    Network,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TaskManagerApp {
    current_window: EWindow,

    processes_sort: EProcessesSort,

    #[serde(skip)]
    sys: System,

    #[serde(skip)]
    refresh_interval: Duration,

    #[serde(skip)]
    last_refresh_time: Instant,
}

impl Default for TaskManagerApp {
    fn default() -> Self {
        Self {
            current_window: EWindow::Processes,
            processes_sort: EProcessesSort::Cpu,
            sys: System::new_all(),
            refresh_interval: Duration::from_secs(1),
            last_refresh_time: Instant::now(),
        }
    }
}

impl TaskManagerApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Set the font color for the dark theme to white.
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::WHITE);
        cc.egui_ctx.set_visuals(visuals);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn bottom_panel(ctx: &Context, current_window: &mut EWindow, sys: &mut System) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        // ui.style_mut().override_text_style = Some(TextStyle::Heading);
                        ui.selectable_value(current_window, EWindow::Processes, "🔢 Processes");
                        ui.selectable_value(current_window, EWindow::Performance, "📈 Performance");
                        ui.selectable_value(current_window, EWindow::AppHistory, "📊 App history");
                        ui.selectable_value(
                            current_window,
                            EWindow::StartupApps,
                            "🏁 Startup apps",
                        );
                        ui.selectable_value(current_window, EWindow::Users, "👥 Users");
                        ui.selectable_value(current_window, EWindow::Details, "📄 Details");
                        ui.selectable_value(current_window, EWindow::Services, "🛠 Services");
                        ui.selectable_value(current_window, EWindow::Settings, "⚙ Settings");
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        #[cfg(target_os = "windows")]
                        {
                            ui.label(format!("Host: {}", sys.host_name().unwrap()));
                            ui.label(format!("User: {}", sys.users().first().unwrap().name()));
                            ui.label(format!(
                                "OS: {} {}",
                                sys.name().unwrap(),
                                sys.os_version().unwrap()
                            ));
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            ui.label(format!("Host: {}", sys.host_name().unwrap()));
                            ui.label(format!("OS: {}", sys.name().unwrap()));
                            ui.label(format!("Version: {}", sys.os_version().unwrap()));
                            ui.label(format!("Kernel: {}", sys.kernel_version().unwrap()));
                        }
                        egui::warn_if_debug_build(ui);
                    });
                });
            });
    }

    fn processes_window(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
            //     ui.heading("Processes");
            //     ui.separator();
            // });

            // Update data
            let now = Instant::now();
            if now - self.last_refresh_time >= self.refresh_interval {
                //sys.refresh_all();
                self.sys.refresh_specifics(
                    RefreshKind::new()
                        //.with_networks()
                        .with_cpu(CpuRefreshKind::new().with_cpu_usage().without_frequency())
                        .with_processes(
                            ProcessRefreshKind::new()
                                .with_cpu()
                                .with_user()
                                .with_disk_usage(), //.without_user(),
                        ),
                );
                self.last_refresh_time = now;
            }

            let processes_sort = &mut self.processes_sort;

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .auto_shrink([false, false])
                .cell_layout(egui::Layout::right_to_left(egui::Align::Center))
                .column(Column::initial(100.0).range(40.0..=300.0)) // Name
                .column(Column::initial(100.0).range(40.0..=300.0)) // User
                .column(Column::initial(100.0).range(50.0..=50.0)) // CPU
                .column(Column::initial(100.0).range(40.0..=300.0)) // Memory
                .column(Column::initial(100.0).range(40.0..=300.0)) // Disk
                .column(Column::initial(100.0).range(40.0..=300.0)) // Network
                .column(Column::remainder()) // Blank
                .min_scrolled_height(0.0)
                .header(40.0, |mut header| {
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                            ui.selectable_value(processes_sort, EProcessesSort::Name, "Name");
                        });
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            ui.heading(" ");
                            ui.selectable_value(processes_sort, EProcessesSort::User, "User");
                        });
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            let cpu = self.sys.global_cpu_info().cpu_usage();
                            ui.heading(format!("{:.0}%", cpu));
                            ui.selectable_value(processes_sort, EProcessesSort::Cpu, "CPU");
                        });
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            let memory = (self.sys.used_memory() as f64
                                / self.sys.total_memory() as f64)
                                * 100.0;
                            ui.heading(format!("{:.0}%", memory));
                            ui.selectable_value(processes_sort, EProcessesSort::Memory, "Memory");
                        });
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            ui.heading(" ");
                            ui.selectable_value(processes_sort, EProcessesSort::Disk, "Disk");
                        });
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                            ui.heading(" ");
                            ui.selectable_value(processes_sort, EProcessesSort::Network, "Network");
                        });
                    });
                })
                .body(|mut body| {
                    let row_height = 28.0;

                    let cpus = self.sys.cpus().len(); //sys.physical_core_count().unwrap();
                    let cpus_f32 = cpus as f32;

                    let mut processes: Vec<&Process> = self.sys.processes().values().collect();

                    match processes_sort {
                        EProcessesSort::Name => {
                            processes.sort_by(|a, b| a.name().cmp(b.name()));
                        }
                        EProcessesSort::User => {
                            processes
                                .sort_by(|a, b| a.user_id().unwrap().cmp(b.user_id().unwrap()));
                        }
                        EProcessesSort::Cpu => {
                            processes
                                .sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap());
                        }
                        EProcessesSort::Memory => {
                            processes.sort_by(|a, b| b.memory().partial_cmp(&a.memory()).unwrap());
                        }
                        EProcessesSort::Disk => {
                            processes.sort_by(|a, b| {
                                (b.disk_usage().read_bytes + b.disk_usage().written_bytes)
                                    .partial_cmp(
                                        &(a.disk_usage().read_bytes + a.disk_usage().written_bytes),
                                    )
                                    .unwrap()
                            });
                        }
                        EProcessesSort::Network => {
                            // TODO
                        }
                    }

                    for process in processes {
                        body.row(row_height, |mut row| {
                            // Name
                            row.col(|ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let name = process.name();
                                        // let title = window_titles.window_title().unwrap();
                                        ui.horizontal(|ui| {
                                            ui.label("▶ 💻");
                                            ui.label(format!("{}", name));
                                        });
                                    },
                                );
                            });
                            // User
                            row.col(|ui| {
                                if let Some(id) = process.user_id() {
                                    if let Some(user) = self.sys.get_user_by_id(id) {
                                        ui.label(user.name());
                                    } else {
                                        ui.label(" ");
                                    }
                                } else {
                                    ui.label(" ");
                                }
                            });
                            // CPU
                            row.col(|ui| {
                                let cpu = process.cpu_usage() / cpus_f32;
                                if cpu < 0.01 {
                                    ui.label("0%");
                                } else {
                                    ui.label(format!("{:.2}%", cpu));
                                }
                            });
                            // Memory
                            row.col(|ui| {
                                let memory = process.memory() as f64 / (1024 * 1024) as f64;
                                if memory < 0.01 {
                                    ui.label("0 MB");
                                } else {
                                    ui.label(format!("{:.2} MB", memory));
                                }
                            });
                            // Disk
                            row.col(|ui| {
                                let disk_read = process.disk_usage().read_bytes;
                                let disk_write = process.disk_usage().written_bytes;
                                let disk_combined = disk_read + disk_write;
                                let disk = disk_combined as f64 / (1024 * 1024) as f64;
                                if disk < 0.01 {
                                    ui.label("0 MB/s");
                                } else {
                                    ui.label(format!("{:.2} MB/s", disk));
                                }
                            });
                            // Network
                            row.col(|ui| {
                                // TODO
                                ui.label("0 Mbps");
                                // if ui.button("kill").clicked() {
                                //     process.kill();
                                // }
                            });
                        });
                    }
                });
        });
    }
}

impl eframe::App for TaskManagerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            current_window,
            processes_sort,
            sys,
            refresh_interval: _,
            last_refresh_time: _,
        } = self;

        if false {
            #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                // The top panel is often a good place for a menu bar:
                egui::menu::bar(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.text_edit_singleline(&mut "Search");
                    });

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                        egui::warn_if_debug_build(ui);
                    });
                });
            });
        }

        Self::bottom_panel(ctx, current_window, sys);

        match current_window {
            EWindow::Processes => {
                Self::processes_window(self, ctx);
            }
            EWindow::Performance => {}
            EWindow::AppHistory => {}
            EWindow::StartupApps => {}
            EWindow::Users => {}
            EWindow::Details => {}
            EWindow::Services => {}
            EWindow::Settings => {}
        }
    }
}
