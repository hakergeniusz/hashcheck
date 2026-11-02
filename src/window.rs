// SPDX-License-Identifier: MIT

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::pango;

use crate::checksum::{self, HashKind, GENERATION_COUNT};
use crate::config::{self, HASH_FUNCTIONS, VERIFICATION_HASH_FUNCTION};
use crate::feedback;
use crate::hash_row::HashRow;
use crate::settings;
use crate::toggle_dialog;
use crate::utils;

/// Files smaller than this are compared against all hash values via content
/// scan, to avoid false-positives when comparing checksum lists.
const MAX_COMPARE_READ_SIZE: u64 = 1000;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Window {
        // Widgets needed across methods.
        pub header_bar: RefCell<Option<adw::HeaderBar>>,
        pub main_stack: RefCell<Option<gtk::Stack>>,
        pub file_info: RefCell<Option<adw::StatusPage>>,
        pub open_file_btn: RefCell<Option<gtk::Button>>,
        pub switcher_bar: RefCell<Option<adw::ViewSwitcherBar>>,
        pub progressbar: RefCell<Option<gtk::ProgressBar>>,
        pub markdown_action: RefCell<Option<gio::SimpleAction>>,
        pub toast_overlay: RefCell<Option<adw::ToastOverlay>>,
        pub tools: RefCell<Option<gtk::Box>>,
        pub verify_overlay_label: RefCell<Option<gtk::Label>>,
        pub verify_text_view: RefCell<Option<gtk::TextView>>,
        pub verify_feedback: RefCell<Option<gtk::Image>>,
        pub compare_btn: RefCell<Option<gtk::Button>>,
        pub compare_btn_stack: RefCell<Option<gtk::Stack>>,
        pub compare_btn_image: RefCell<Option<gtk::Image>>,
        pub compare_btn_label: RefCell<Option<gtk::Label>>,
        pub drop_overlay_revealer: RefCell<Option<gtk::Revealer>>,
        pub main_dnd: RefCell<Option<gtk::DropTarget>>,
        pub compare_dnd: RefCell<Option<gtk::DropTarget>>,
        pub results_stack: RefCell<Option<adw::ViewStack>>,

        // State.
        pub hash_rows: RefCell<HashMap<HashKind, HashRow>>,
        pub hash_results: RefCell<HashMap<HashKind, String>>,
        pub file_path_queued: RefCell<Option<PathBuf>>,
        pub working: Cell<bool>,
        pub done: Cell<u32>,
        pub total: Cell<u32>,
    }

    impl imp::Window {
        pub(super) fn is_spinner_visible(&self) -> bool {
            self.main_stack
                .borrow()
                .as_ref()
                .and_then(|s| s.visible_child_name())
                .is_some_and(|name| name == "spinner")
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "HashcheckWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_widgets();
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, adw::Window, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

/// Creates a window, restores its geometry and shows it.
/// If a file is passed, it will be hashed right away.
pub fn open_window_with_file(app: &adw::Application, file: Option<gio::File>) {
    let window = Window::new(app);

    let (width, height, maximized) = settings::with(|s| s.window_settings());
    window.set_default_size(width, height);
    if maximized {
        window.maximize();
    }

    window.present();

    if let Some(path) = file.and_then(|f| f.path()).filter(|p| p.is_file()) {
        window.loading();
        window.set_file(path);
    }
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn setup_widgets(&self) {
        let imp = self.imp();

        // -- Header bar ---------------------------------------------------
        let open_file_content = adw::ButtonContent::builder()
            .icon_name("document-open-symbolic")
            .label("_Open")
            .use_underline(true)
            .tooltip_text("Open…")
            .build();
        let open_file_btn = gtk::Button::builder()
            .visible(false)
            .child(&open_file_content)
            .css_classes(["flat"])
            .build();

        let results_stack = adw::ViewStack::builder()
            .vhomogeneous(false)
            .hhomogeneous(false)
            .build();

        let header_bar = adw::HeaderBar::builder().show_title(false).build();
        header_bar.pack_start(&open_file_btn);
        header_bar.set_title_widget(Some(
            &adw::ViewSwitcher::builder()
                .policy(adw::ViewSwitcherPolicy::Wide)
                .stack(&results_stack)
                .build(),
        ));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .primary(true)
            .menu_model(&primary_menu())
            .build();
        header_bar.pack_end(&menu_button);

        // -- Hash rows ----------------------------------------------------
        let hash_row_container = gtk::ListBox::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        for (kind, name) in HASH_FUNCTIONS {
            let obj = self.downgrade();
            let row = HashRow::new(*kind, name, move |kind| {
                if let Some(obj) = obj.upgrade() {
                    obj.copy_hash(kind);
                }
            });
            hash_row_container.append(row.widget());
            imp.hash_rows.borrow_mut().insert(*kind, row);
        }

        // -- Verify page ----------------------------------------------------
        let verify_overlay_label = gtk::Label::builder()
            .label("MD5,SHA-1,SHA-256,SHA-512,Blake3,CRC32 or Adler32 Hash")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .justify(gtk::Justification::Fill)
            .margin_end(22)
            .margin_start(14)
            .margin_top(11)
            .wrap(true)
            .wrap_mode(pango::WrapMode::Char)
            .sensitive(false)
            .visible(false)
            .css_classes(["monospace"])
            .build();

        let verify_feedback = gtk::Image::builder()
            .pixel_size(16)
            .icon_name("hashcheck-success-symbolic")
            .halign(gtk::Align::End)
            .valign(gtk::Align::End)
            .margin_end(5)
            .margin_bottom(5)
            .sensitive(false)
            .visible(false)
            .build();

        let verify_text_view = gtk::TextView::builder()
            .top_margin(11)
            .bottom_margin(11)
            .left_margin(5)
            .right_margin(5)
            .height_request(125)
            .wrap_mode(gtk::WrapMode::Char)
            .accepts_tab(false)
            .input_purpose(gtk::InputPurpose::FreeForm)
            .input_hints(gtk::InputHints::NO_SPELLCHECK | gtk::InputHints::NO_EMOJI)
            .tooltip_text("Insert a MD5/SHA-1/SHA-256/SHA-512/Blake3/CRC32/Adler32 Hash")
            .css_classes(["card-like", "monospace"])
            .build();

        let verify_scroller = gtk::ScrolledWindow::builder()
            .height_request(125)
            .child(&verify_text_view)
            .build();
        let verify_overlay = gtk::Overlay::builder()
            .overflow(gtk::Overflow::Hidden)
            .child(&verify_scroller)
            .build();
        verify_overlay.add_overlay(&verify_feedback);
        verify_overlay.add_overlay(&verify_overlay_label);

        let checksum_title = title_label("Checksum");
        let verify_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .build();
        verify_box.append(&checksum_title);
        verify_box.append(&verify_overlay);

        let compare_btn_stack = gtk::Stack::new();
        let compare_btn_image = gtk::Image::builder()
            .pixel_size(36)
            .icon_name("hashcheck-file-symbolic")
            .build();
        compare_btn_stack.add_named(&compare_btn_image, Some("image"));
        compare_btn_stack.add_named(&adw::Spinner::new(), Some("spinner"));

        let compare_btn_label = gtk::Label::builder()
            .label("Choose File…")
            .max_width_chars(15)
            .ellipsize(pango::EllipsizeMode::End)
            .build();

        let compare_inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .spacing(12)
            .build();
        compare_inner.append(&compare_btn_stack);
        compare_inner.append(&compare_btn_label);

        let compare_dnd =
            gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let compare_btn = gtk::Button::builder()
            .tooltip_text("Select Another File to Check Against")
            .height_request(125)
            .use_underline(true)
            .child(&compare_inner)
            .css_classes(["card-like"])
            .build();
        compare_btn.add_controller(compare_dnd.clone());

        let file_title = title_label("File");
        let compare_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .build();
        compare_box.append(&file_title);
        compare_box.append(&compare_btn);

        let tools = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(true)
            .spacing(45)
            .build();
        tools.append(&verify_box);
        tools.append(&compare_box);

        // -- Results page ---------------------------------------------------
        results_stack.add_titled_with_icon(
            &hash_row_container,
            Some("hashes"),
            "Hash",
            "hashcheck-hash-symbolic",
        );
        results_stack.add_titled_with_icon(
            &tools,
            Some("verify"),
            "Verify",
            "hashcheck-success-symbolic",
        );

        let file_info = adw::StatusPage::builder()
            .vexpand(true)
            .child(
                &adw::Clamp::builder()
                    .tightening_threshold(100)
                    .child(&results_stack)
                    .build(),
            )
            .build();

        // -- Spinner page ---------------------------------------------------
        let progressbar = gtk::ProgressBar::builder()
            .show_text(true)
            .text("Pending")
            .valign(gtk::Align::Center)
            .hexpand(true)
            .vexpand(true)
            .build();
        let spinner_page = adw::StatusPage::builder()
            .title("Calculating Hashes")
            .description("This might take a while")
            .child(
                &adw::Clamp::builder()
                    .tightening_threshold(100)
                    .maximum_size(400)
                    .child(&progressbar)
                    .build(),
            )
            .build();

        // -- Welcome page ---------------------------------------------------
        let welcome_btn = gtk::Button::builder()
            .halign(gtk::Align::Center)
            .label("_Open a File")
            .use_underline(true)
            .css_classes(["suggested-action", "pill"])
            .build();
        let welcomer = adw::StatusPage::builder()
            .vexpand(true)
            .icon_name(config::APP_ID)
            .title("Hashcheck")
            .description("Check hashes for your files")
            .child(&welcome_btn)
            .css_classes(["icon-dropshadow"])
            .build();

        // -- Main stack + drop targets ---------------------------------------
        let main_stack = gtk::Stack::builder().css_classes(["no-drop"]).build();
        main_stack.add_named(&welcomer, Some("welcomer"));
        main_stack.add_named(&spinner_page, Some("spinner"));
        main_stack.add_named(&file_info, Some("results"));

        let main_dnd = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        main_stack.add_controller(main_dnd.clone());

        // -- Drop overlay -----------------------------------------------------
        let drop_overlay_revealer = gtk::Revealer::builder()
            .can_target(false)
            .transition_type(gtk::RevealerTransitionType::Crossfade)
            .transition_duration(600)
            .child(
                &adw::StatusPage::builder()
                    .title("Drop to hash")
                    .icon_name("hashcheck-file-symbolic")
                    .css_classes(["status"])
                    .build(),
            )
            .build();

        // -- Assemble ----------------------------------------------------------
        let switcher_bar = adw::ViewSwitcherBar::builder()
            .stack(&results_stack)
            .visible(false)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.add_bottom_bar(&switcher_bar);
        toolbar_view.set_content(Some(&main_stack));

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        let overlay = gtk::Overlay::new();
        overlay.add_overlay(&drop_overlay_revealer);
        overlay.set_child(Some(&toast_overlay));

        let bin = adw::Bin::builder()
            .css_name("dropoverlay")
            .child(&overlay)
            .build();

        self.set_width_request(360);
        self.set_height_request(360);
        self.set_title(Some("Hashcheck"));
        adw::prelude::AdwApplicationWindowExt::set_content(self, Some(&bin));

        // -- Breakpoint ---------------------------------------------------------
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            400.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.connect_apply(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_| {
                let imp = obj.imp();
                if let Some(hb) = imp.header_bar.borrow().as_ref() {
                    hb.set_title_widget(Option::<&gtk::Widget>::None);
                }
                if let Some(sb) = imp.switcher_bar.borrow().as_ref() {
                    sb.set_reveal(true);
                }
                if let Some(tools) = imp.tools.borrow().as_ref() {
                    tools.set_orientation(gtk::Orientation::Vertical);
                    tools.set_spacing(32);
                }
            }
        ));
        breakpoint.connect_unapply(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_| {
                let imp = obj.imp();
                if let Some(hb) = imp.header_bar.borrow().as_ref() {
                    if let Some(stack) = imp.results_stack.borrow().as_ref() {
                        let switcher = adw::ViewSwitcher::builder()
                            .policy(adw::ViewSwitcherPolicy::Wide)
                            .stack(stack)
                            .build();
                        hb.set_title_widget(Some(&switcher));
                    }
                }
                if let Some(sb) = imp.switcher_bar.borrow().as_ref() {
                    sb.set_reveal(false);
                }
                if let Some(tools) = imp.tools.borrow().as_ref() {
                    tools.set_orientation(gtk::Orientation::Horizontal);
                    tools.set_spacing(45);
                }
            }
        ));
        adw::prelude::AdwApplicationWindowExt::add_breakpoint(self, breakpoint);

        // -- Store fields ---------------------------------------------------------
        *imp.header_bar.borrow_mut() = Some(header_bar);
        *imp.main_stack.borrow_mut() = Some(main_stack);
        *imp.file_info.borrow_mut() = Some(file_info);
        *imp.open_file_btn.borrow_mut() = Some(open_file_btn);
        *imp.switcher_bar.borrow_mut() = Some(switcher_bar);
        *imp.progressbar.borrow_mut() = Some(progressbar);
        *imp.toast_overlay.borrow_mut() = Some(toast_overlay);
        *imp.tools.borrow_mut() = Some(tools);
        *imp.verify_overlay_label.borrow_mut() = Some(verify_overlay_label);
        *imp.verify_text_view.borrow_mut() = Some(verify_text_view);
        *imp.verify_feedback.borrow_mut() = Some(verify_feedback);
        *imp.compare_btn.borrow_mut() = Some(compare_btn);
        *imp.compare_btn_stack.borrow_mut() = Some(compare_btn_stack);
        *imp.compare_btn_image.borrow_mut() = Some(compare_btn_image);
        *imp.compare_btn_label.borrow_mut() = Some(compare_btn_label);
        *imp.drop_overlay_revealer.borrow_mut() = Some(drop_overlay_revealer);
        *imp.main_dnd.borrow_mut() = Some(main_dnd);
        *imp.compare_dnd.borrow_mut() = Some(compare_dnd);
        *imp.results_stack.borrow_mut() = Some(results_stack);

        self.update_hashrow_visibility();
        self.setup_signals(welcome_btn);
    }

    fn setup_signals(&self, welcome_btn: gtk::Button) {
        let imp = self.imp();

        // Window actions.
        let open_file_action = gio::SimpleAction::new("open-file", None);
        open_file_action.connect_activate(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_, _| obj.on_open_btn_clicked()
        ));
        self.add_action(&open_file_action);

        let markdown_action = gio::SimpleAction::new("markdown", None);
        markdown_action.set_enabled(false);
        markdown_action.connect_activate(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_, _| obj.on_markdown()
        ));
        self.add_action(&markdown_action);
        *imp.markdown_action.borrow_mut() = Some(markdown_action);

        let toggle_action = gio::SimpleAction::new("toggle-hashes", None);
        toggle_action.connect_activate(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_, _| toggle_dialog::show(&obj)
        ));
        self.add_action(&toggle_action);

        // Buttons.
        welcome_btn.connect_clicked(glib::clone!(
            #[weak(rename_to = obj)] self,
            move |_| obj.on_open_btn_clicked()
        ));
        if let Some(btn) = imp.open_file_btn.borrow().as_ref().cloned() {
            btn.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)] self,
                move |_| obj.on_open_btn_clicked()
            ));
        }

        // Verify input.
        if let Some(text_view) = imp.verify_text_view.borrow().as_ref().cloned() {
            let buffer = text_view.buffer();
            buffer.connect_notify_local(
                Some("text"),
                glib::clone!(
                    #[weak(rename_to = obj)] self,
                    move |buffer, _| {
                        let (start, end) = buffer.bounds();
                        let text = buffer.text(&start, &end, true);
                        obj.handle_input_change(&text);
                    }
                ),
            );
        }

        // Compare button.
        if let Some(btn) = imp.compare_btn.borrow().as_ref().cloned() {
            btn.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)] self,
                move |_| {
                    let dialog = gtk::FileDialog::builder()
                        .title("Choose a File")
                        .modal(true)
                        .build();
                    let obj_w = obj.downgrade();
                    dialog.open(Some(&obj), gio::Cancellable::NONE, move |res| {
                        if let (Ok(file), Some(obj)) = (res, obj_w.upgrade()) {
                            obj.compare_file(file);
                        }
                    });
                }
            ));
        }

        // Main drop target: hash the file.
        if let Some(dnd) = imp.main_dnd.borrow().as_ref().cloned() {
            dnd.connect_enter(glib::clone!(
                #[weak(rename_to = obj)] self,
                #[upgrade_or] gdk::DragAction::COPY,
                move |_, _, _| {
                    let imp = obj.imp();
                    let is_spinner = imp.is_spinner_visible();
                    if let Some(revealer) = imp.drop_overlay_revealer.borrow().as_ref() {
                        revealer.set_reveal_child(!is_spinner);
                    }
                    gdk::DragAction::COPY
                }
            ));
            dnd.connect_leave(glib::clone!(
                #[weak(rename_to = obj)] self,
                move |_| {
                    if let Some(revealer) =
                        obj.imp().drop_overlay_revealer.borrow().as_ref()
                    {
                        revealer.set_reveal_child(false);
                    }
                }
            ));
            dnd.connect_drop(glib::clone!(
                #[weak(rename_to = obj)] self,
                #[upgrade_or] false,
                move |_, value, _, _| {
                    let imp = obj.imp();
                    if imp.is_spinner_visible() {
                        return false;
                    }
                    if let Ok(files) = value.get::<gdk::FileList>() {
                        let file = files
                            .files()
                            .into_iter()
                            .find(|f| f.uri().to_lowercase().starts_with("file://"));
                        if let Some(file) = file {
                            obj.on_drop(file);
                            return true;
                        }
                    }
                    false
                }
            ));
        }

        // Compare drop target: only reveal/unreveal the overlay.
        if let Some(dnd) = imp.compare_dnd.borrow().as_ref().cloned() {
            dnd.connect_enter(glib::clone!(
                #[weak(rename_to = obj)] self,
                #[upgrade_or] gdk::DragAction::COPY,
                move |_, _, _| {
                    if let Some(revealer) =
                        obj.imp().drop_overlay_revealer.borrow().as_ref()
                    {
                        revealer.set_reveal_child(false);
                    }
                    gdk::DragAction::COPY
                }
            ));
            dnd.connect_leave(glib::clone!(
                #[weak(rename_to = obj)] self,
                move |_| {
                    if let Some(revealer) =
                        obj.imp().drop_overlay_revealer.borrow().as_ref()
                    {
                        revealer.set_reveal_child(true);
                    }
                }
            ));
            dnd.connect_drop(glib::clone!(
                #[weak(rename_to = obj)] self,
                #[upgrade_or] false,
                move |_, value, _, _| {
                    if let Ok(files) = value.get::<gdk::FileList>() {
                        let file = files
                            .files()
                            .into_iter()
                            .find(|f| f.uri().to_lowercase().starts_with("file://"));
                        if let Some(file) = file {
                            obj.compare_file(file);
                            return true;
                        }
                    }
                    false
                }
            ));
        }

        // Close request: save geometry, hide while hashing if other windows exist.
        self.connect_close_request(glib::clone!(
            #[weak(rename_to = obj)] self,
            #[upgrade_or] glib::Propagation::Proceed,
            move |_| {
                let windows = obj
                    .application()
                    .and_then(|app| app.downcast::<adw::Application>().ok())
                    .map(|app| app.windows().len())
                    .unwrap_or(0);
                if obj.imp().working.get() && windows > 1 {
                    obj.set_hide_on_close(true);
                } else {
                    settings::with(|s| s.save_window(obj.upcast_ref()));
                }
                glib::Propagation::Proceed
            }
        ));
    }

    // -- File handling --------------------------------------------------------

    pub fn continue_queue(&self) {
        let imp = self.imp();
        let queued = imp.file_path_queued.borrow_mut().take();
        if let Some(path) = queued {
            self.set_file(path);
        }
    }

    pub fn set_file(&self, path: PathBuf) {
        let imp = self.imp();

        if GENERATION_COUNT.load(Ordering::SeqCst) > 0 {
            imp.file_path_queued.replace(Some(path));
            return;
        }

        imp.file_path_queued.replace(None);
        GENERATION_COUNT.fetch_add(1, Ordering::SeqCst);
        imp.done.set(0);
        imp.working.set(true);

        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        if let Some(info) = imp.file_info.borrow().as_ref() {
            info.set_title(&title);
            info.set_description(Some(&utils::real_path(&path)));
        }

        let disabled = settings::with(|s| s.disabled_hash_functions());
        let enabled: Vec<HashKind> = HASH_FUNCTIONS
            .iter()
            .map(|(kind, _)| *kind)
            .filter(|kind| !disabled.contains(kind))
            .collect();
        let total = enabled.len() as u32;
        imp.total.set(total);

        if let Some(pb) = imp.progressbar.borrow().as_ref() {
            pb.set_fraction(0.0);
            pb.set_text(Some(&format!("0 of {total} hashes calculated")));
        }

        let (tx, rx) = async_channel::unbounded::<checksum::HashEvent>();
        for kind in enabled {
            let tx = tx.clone();
            let p = path.clone();
            std::thread::spawn(move || {
                // Only plain data crosses the thread boundary: GObject wrappers
                // are not Send, the UI is updated from a main-context future.
                let result = checksum::hash_file(kind, &p);
                let _ = tx.send_blocking(checksum::HashEvent::Done(kind, result));
            });
        }
        drop(tx);

        let obj = self.downgrade();
        glib::MainContext::default().spawn_local(async move {
            while let Ok(event) = rx.recv().await {
                let Some(obj) = obj.upgrade() else {
                    break;
                };
                obj.handle_hash_event(event);
            }
        });
    }

    fn handle_hash_event(&self, event: checksum::HashEvent) {
        let imp = self.imp();
        match event {
            checksum::HashEvent::Done(kind, Ok(value)) => {
                imp.hash_results.borrow_mut().insert(kind, value.clone());
                if let Some(row) = imp.hash_rows.borrow().get(&kind) {
                    row.set_hash(&value);
                }
                let done = imp.done.get() + 1;
                imp.done.set(done);
                let total = imp.total.get().max(1);
                if let Some(pb) = imp.progressbar.borrow().as_ref() {
                    pb.set_fraction((done as f64 / total as f64).min(1.0));
                    pb.set_text(Some(&format!("{done} of {total} hashes calculated")));
                }
                if done >= imp.total.get() {
                    self.finish_generation();
                }
            }
            checksum::HashEvent::Done(_, Err(err)) => {
                self.error_generation(&format!("Failed to hash the file: {err}"));
            }
        }
    }

    fn finish_generation(&self) {
        let imp = self.imp();
        self.update_hashrow_visibility();
        if let Some(stack) = imp.main_stack.borrow().as_ref() {
            stack.set_visible_child_name("results");
        }
        if let Some(action) = imp.markdown_action.borrow().as_ref() {
            action.set_enabled(true);
        }
        if let Some(hb) = imp.header_bar.borrow().as_ref() {
            hb.set_show_title(true);
        }
        if let Some(btn) = imp.open_file_btn.borrow().as_ref() {
            btn.set_visible(true);
        }
        if let Some(bar) = imp.switcher_bar.borrow().as_ref() {
            bar.set_visible(true);
        }
        self.flow_queue();
    }

    fn error_generation(&self, message: &str) {
        let imp = self.imp();
        if let Some(overlay) = imp.toast_overlay.borrow().as_ref() {
            overlay.add_toast(adw::Toast::builder().title(message).timeout(5).build());
        }
        if let Some(stack) = imp.main_stack.borrow().as_ref() {
            stack.set_visible_child_name("welcomer");
        }
        self.flow_queue();
    }

    /// Unlocks the window for the next generation and continues queued files.
    fn flow_queue(&self) {
        let imp = self.imp();
        imp.working.set(false);
        GENERATION_COUNT.fetch_sub(1, Ordering::SeqCst);

        if let Some(app) = self
            .application()
            .and_then(|app| app.downcast::<adw::Application>().ok())
        {
            for window in app.windows() {
                if let Some(win) = window.downcast_ref::<Window>() {
                    win.continue_queue();
                }
            }
        }

        if !self.is_visible() {
            self.destroy();
        }
    }

    pub fn update_hashrow_visibility(&self) {
        let disabled = settings::with(|s| s.disabled_hash_functions());
        for (kind, row) in self.imp().hash_rows.borrow().iter() {
            row.set_visible_hash(!disabled.contains(kind));
        }
    }

    // -- UI state helpers ------------------------------------------------------

    pub fn loading(&self) {
        let imp = self.imp();
        if let Some(pb) = imp.progressbar.borrow().as_ref() {
            pb.set_text(Some("Pending"));
            pb.set_fraction(0.0);
        }
        if let Some(stack) = imp.main_stack.borrow().as_ref() {
            stack.set_visible_child_name("spinner");
        }
        if let Some(action) = imp.markdown_action.borrow().as_ref() {
            action.set_enabled(false);
        }
        if let Some(hb) = imp.header_bar.borrow().as_ref() {
            hb.set_show_title(false);
        }
        if let Some(btn) = imp.open_file_btn.borrow().as_ref() {
            btn.set_visible(false);
        }
        if let Some(bar) = imp.switcher_bar.borrow().as_ref() {
            bar.set_visible(false);
        }
        self.reset_feedback();
    }

    fn reset_feedback(&self) {
        let imp = self.imp();
        if let Some(tv) = imp.verify_text_view.borrow().as_ref() {
            tv.buffer().set_text("");
        }
        if let Some(stack) = imp.compare_btn_stack.borrow().as_ref() {
            stack.set_visible_child_name("image");
        }
        if let Some(label) = imp.compare_btn_label.borrow().as_ref() {
            label.set_text("Choose File…");
        }
        if let Some(btn) = imp.compare_btn.borrow().as_ref() {
            btn.remove_css_class("success");
            btn.remove_css_class("error");
        }
        if let Some(image) = imp.compare_btn_image.borrow().as_ref() {
            image.set_icon_name(Some("hashcheck-file-symbolic"));
            image.set_tooltip_text(Some(""));
        }
    }

    fn handle_input_change(&self, text: &str) {
        let imp = self.imp();
        let Some(tv) = imp.verify_text_view.borrow().as_ref().cloned() else {
            return;
        };
        let Some(fb) = imp.verify_feedback.borrow().as_ref().cloned() else {
            return;
        };
        let Some(overlay_label) = imp.verify_overlay_label.borrow().as_ref().cloned() else {
            return;
        };

        let normalized = utils::normalize_input(text);
        if normalized.is_empty() {
            overlay_label.set_visible(true);
            fb.set_visible(false);
            tv.remove_css_class("success");
            tv.remove_css_class("error");
            tv.set_tooltip_text(Some(
                "Insert a MD5/SHA-1/SHA-256/SHA-512/Blake3/CRC32/Adler32 Hash",
            ));
            return;
        }

        overlay_label.set_visible(false);
        fb.set_visible(true);

        let result = imp
            .hash_results
            .borrow()
            .values()
            .any(|value| value == &normalized);

        tv.add_css_class(feedback::css_add(result));
        tv.remove_css_class(feedback::css_remove(result));

        fb.set_icon_name(Some(feedback::icon(result)));
        fb.add_css_class(feedback::css_add(result));
        fb.remove_css_class(feedback::css_remove(result));

        let title = feedback::title(result);
        tv.set_tooltip_text(Some(title));
        tv.announce(title, gtk::AccessibleAnnouncementPriority::High);
    }

    pub fn on_drop(&self, file: gio::File) {
        self.loading();
        if let Some(path) = file.path().filter(|p| p.is_file()) {
            self.set_file(path);
        }
    }

    pub fn on_open_btn_clicked(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Choose a File")
            .modal(true)
            .build();
        let obj = self.downgrade();
        dialog.open_multiple(Some(self), gio::Cancellable::NONE, move |res| {
            let Ok(files) = res else {
                return; // cancelled
            };
            let Some(obj) = obj.upgrade() else {
                return;
            };

            let gio_files: Vec<gio::File> = (0..files.n_items())
                .filter_map(|i| files.item(i))
                .filter_map(|o| o.downcast::<gio::File>().ok())
                .collect();

            if let Some(first) = gio_files.first().cloned() {
                obj.loading();
                if let Some(path) = first.path().filter(|p| p.is_file()) {
                    obj.set_file(path);
                }
            }
            // Remaining files open in their own windows.
            if gio_files.len() > 1 {
                if let Some(app) = obj
                    .application()
                    .and_then(|app| app.downcast::<adw::Application>().ok())
                {
                    for file in gio_files.iter().skip(1) {
                        open_window_with_file(&app, Some(file.clone()));
                    }
                }
            }
        });
    }

    pub fn copy_hash(&self, kind: HashKind) {
        if let Some(value) = self.imp().hash_results.borrow().get(&kind) {
            self.clipboard().set_text(value);
        }
    }

    pub fn on_markdown(&self) {
        let imp = self.imp();
        let results = imp.hash_results.borrow();
        if results.is_empty() {
            return;
        }

        let max_k = results
            .keys()
            .map(|k| config::hash_display_name(*k).len())
            .max()
            .unwrap_or(3)
            .max(3);
        let max_v = results
            .values()
            .map(|v| v.len() + 2)
            .max()
            .unwrap_or(3)
            .max(3);

        let mut table = String::new();
        table.push('|');
        table.push_str(&format!("{:^width$}", "CRF", width = max_k));
        table.push('|');
        table.push_str(&format!("{:^width$}", "Value", width = max_v));
        table.push('|');
        table.push('\n');
        table.push('|');
        table.push_str(&"-".repeat(max_k));
        table.push('|');
        table.push_str(&"-".repeat(max_v));
        table.push('|');
        table.push('\n');
        for (kind, name) in HASH_FUNCTIONS {
            let Some(value) = results.get(kind) else {
                continue;
            };
            table.push('|');
            table.push_str(&format!("{:<width$}", name, width = max_k));
            table.push('|');
            table.push_str(&format!("{:<width$}", format!("`{value}`"), width = max_v));
            table.push('|');
            table.push('\n');
        }

        self.clipboard().set_text(table.trim());
    }

    // -- Compare tool -----------------------------------------------------------

    pub fn compare_file(&self, file: gio::File) {
        let Some(path) = file.path().filter(|p| p.is_file()) else {
            return;
        };
        let imp = self.imp();

        if let Some(stack) = imp.compare_btn_stack.borrow().as_ref() {
            stack.set_visible_child_name("spinner");
        }
        if let Some(btn) = imp.compare_btn.borrow().as_ref() {
            btn.remove_css_class("success");
            btn.remove_css_class("error");
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        if let Some(label) = imp.compare_btn_label.borrow().as_ref() {
            label.set_text(&name);
            label.set_tooltip_text(Some(&name));
        }

        let stored_sha = imp
            .hash_results
            .borrow()
            .get(&VERIFICATION_HASH_FUNCTION)
            .cloned();
        let needles: Vec<String> = imp.hash_results.borrow().values().cloned().collect();

        let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX);
        let (tx, rx) = async_channel::bounded::<bool>(1);
        let tx_worker = tx.clone();
        std::thread::spawn(move || {
            let result = checksum::hash_file(VERIFICATION_HASH_FUNCTION, &path)
                .map(|ver| {
                    let mut result = stored_sha.as_deref() == Some(ver.as_str());
                    if !result && len < MAX_COMPARE_READ_SIZE {
                        result = utils::compare_content(&path, &needles);
                    }
                    result
                })
                .unwrap_or(false);

            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = tx_worker.send_blocking(result);
        });
        drop(tx);

        let obj = self.downgrade();
        glib::MainContext::default().spawn_local(async move {
            if let (Ok(result), Some(obj)) = (rx.recv().await, obj.upgrade()) {
                obj.apply_compare_result(result);
            }
        });
    }

    fn apply_compare_result(&self, result: bool) {
        let imp = self.imp();
        if let Some(stack) = imp.compare_btn_stack.borrow().as_ref() {
            stack.set_visible_child_name("image");
        }
        if let Some(image) = imp.compare_btn_image.borrow().as_ref() {
            image.set_icon_name(Some(feedback::icon(result)));
            image.set_tooltip_text(Some(feedback::title(result)));
        }
        if let Some(btn) = imp.compare_btn.borrow().as_ref() {
            btn.add_css_class(feedback::css_add(result));
            btn.remove_css_class(feedback::css_remove(result));
            btn.announce(feedback::title(result), gtk::AccessibleAnnouncementPriority::High);
        }
    }
}

fn title_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .wrap_mode(pango::WrapMode::WordChar)
        .justify(gtk::Justification::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .css_classes(["title", "title-1", "compact-title"])
        .build()
}

fn primary_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append(Some("_New Window"), Some("app.new-window"));
    menu.append(Some("_Compare Hash Functions"), Some("app.hashinfo"));

    let markdown = gio::MenuItem::new(Some("Copy as _Markdown Table"), Some("win.markdown"));
    markdown.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    menu.append_item(&markdown);

    menu.append(Some("_Toggle Hash Functions"), Some("win.toggle-hashes"));
    menu.append(Some("_Keyboard Shortcuts"), Some("app.shortcuts"));
    menu.append(Some("_About Hashcheck"), Some("app.about"));
    menu
}
