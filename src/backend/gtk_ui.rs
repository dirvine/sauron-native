use crate::{Backend, Component, Widget};
use gdk_pixbuf::{PixbufLoader, PixbufLoaderExt};
use gio::{prelude::*, ApplicationFlags};
use glib::Value;
use gtk::{
    prelude::*, Application, ApplicationWindow, Button, CheckButton, Container, CssProvider, Entry,
    EntryBuffer, Image, Orientation, StyleContext, TextBuffer, TextBufferExt, TextTagTable,
    TextView, WidgetExt, Window, WindowPosition, WindowType,
};
use std::{fmt::Debug, marker::PhantomData, rc::Rc};

use crate::{Attribute, Node, Patch};
use gtk::{IsA, Label, Paned};
use sauron_vdom::{
    event::{InputEvent, MouseEvent},
    AttribValue, Dispatch,
};
use std::cell::RefCell;

mod apply_patches;

pub struct GtkBackend<APP, MSG>
where
    MSG: 'static,
{
    app: Rc<RefCell<APP>>,
    current_vdom: Rc<RefCell<Node<MSG>>>,
    root_node: Rc<RefCell<Option<GtkWidget>>>,
    application: Application,
    _phantom_msg: PhantomData<MSG>,
}
impl<APP, MSG> GtkBackend<APP, MSG>
where
    MSG: Debug + 'static,
    APP: Component<MSG> + 'static,
{
    fn new(app: APP) -> Rc<Self> {
        let current_vdom = app.view();
        let root_vdom = app.view();

        if gtk::init().is_err() {
            println!("failed to initialize GTK Application");
        }
        let root_widget: Option<GtkWidget> = None;
        let mut backend = GtkBackend {
            app: Rc::new(RefCell::new(app)),
            current_vdom: Rc::new(RefCell::new(current_vdom)),
            root_node: Rc::new(RefCell::new(root_widget)),
            application: Application::new("ivanceras.github.io.gtk", ApplicationFlags::FLAGS_NONE)
                .expect("Failed to start app"),
            _phantom_msg: PhantomData,
        };
        let rc_backend = Rc::new(backend);
        let root_widget = Self::node_tree_to_gtk(&rc_backend, root_vdom);
        *rc_backend.root_node.borrow_mut() = Some(root_widget);
        rc_backend
    }

    fn root_container(self: &Rc<Self>) -> Rc<Container> {
        let root_widget = self.root_node.borrow();
        if let Some(root_widget) = &*root_widget {
            match &root_widget {
                GtkWidget::GBox(gbox) => {
                    let container: &Container = gbox.upcast_ref();
                    Rc::new(container.clone())
                }
                _ => panic!("expecting it to be a container"),
            }
        } else {
            panic!("must have a root widget");
        }
    }

    fn dispatch_inner(self: &Rc<Self>, msg: MSG)
    where
        MSG: Debug,
    {
        println!("dispatching : {:?}", msg);
        self.app.borrow_mut().update(msg);
        let new_view = self.app.borrow().view();
        {
            let current_vdom = self.current_vdom.borrow();
            let diff = sauron_vdom::diff(&current_vdom, &new_view);
            println!("diff: {:#?}", diff);
            apply_patches::apply_patches(&self.root_container(), &diff);
        }
        *self.current_vdom.borrow_mut() = new_view;
    }

    fn create_app(mut self: &Rc<Self>)
    where
        APP: Component<MSG> + 'static,
        MSG: Clone + Debug + 'static,
    {
        let self_clone = Rc::clone(&self);
        self.application.connect_activate(move |uiapp| {
            let win = ApplicationWindow::new(uiapp);
            let rc_win = Rc::new(win);
            rc_win.set_default_size(800, 1000);
            rc_win.set_icon_name(Some("applications-graphics"));
            rc_win.set_title("Gtk backend");
            self_clone.attach_root_widget(&rc_win);
            rc_win.show_all();
        });
        self.application.run(&[]);
    }

    fn attach_root_widget(self: &Rc<Self>, window: &Rc<ApplicationWindow>)
    where
        APP: Component<MSG> + 'static,
        MSG: Clone + Debug + 'static,
    {
        if let Some(root_widget) = self.root_node.borrow().as_ref() {
            if let Some(root_widget) = root_widget.as_widget() {
                window.add(root_widget);
            }
        }
    }

    fn node_tree_to_gtk<DSP>(program: &Rc<DSP>, widget_node: crate::Node<MSG>) -> GtkWidget
    where
        MSG: Debug + 'static,
        DSP: Dispatch<MSG> + 'static,
    {
        match widget_node {
            crate::Node::Element(element) => {
                let mut gtk_widget = Self::node_to_gtk(program, element.tag, &element.attrs);
                let mut children = vec![];
                for child in element.children {
                    let gtk_child = Self::node_tree_to_gtk(program, child);
                    children.push(gtk_child);
                }
                gtk_widget.add_children(children);
                gtk_widget
            }
            crate::Node::Text(txt) => Button::new_with_label(&txt.text).into(),
        }
    }

    fn node_to_gtk<DSP>(program: &Rc<DSP>, widget: Widget, attrs: &Vec<Attribute<MSG>>) -> GtkWidget
    where
        MSG: Debug + 'static,
        DSP: Dispatch<MSG> + 'static,
    {
        match widget {
            Widget::Vbox => {
                let vbox = gtk::Box::new(Orientation::Vertical, 0);
                vbox.into()
            }
            Widget::Hbox => gtk::Box::new(Orientation::Horizontal, 0).into(),
            Widget::Button => {
                let txt: String = if let Some(attr) = attrs.iter().find(|attr| attr.name == "value")
                {
                    if let Some(value) = attr.get_value() {
                        value.to_string()
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };
                let btn = Button::new_with_label(&txt);
                for attr in attrs {
                    match &attr.value {
                        AttribValue::Callback(cb) => match attr.name {
                            "click" => {
                                let program = Rc::clone(program);
                                let cb_clone = cb.clone();
                                btn.connect_clicked(move |_| {
                                    let mouse_event = MouseEvent::default();
                                    let msg = cb_clone.emit(mouse_event);
                                    println!("got msg: {:?}", msg);
                                    //TODO: set the current_vdom after dispatching the callback
                                    program.dispatch(msg);
                                });
                            }
                            _ => {}
                        },
                        _ => (),
                    }
                }
                btn.into()
            }
            Widget::Text(txt) => textview(&txt),
            Widget::TextInput(txt) => {
                let buffer = EntryBuffer::new(Some(&*txt));
                let entry = Entry::new_with_buffer(&buffer);

                for attr in attrs {
                    match &attr.value {
                        AttribValue::Callback(cb) => match attr.name {
                            "input" => {
                                let program = Rc::clone(program);
                                let cb_clone = cb.clone();
                                entry.connect_property_text_notify(move |entry| {
                                    let input_event =
                                        InputEvent::new(entry.get_buffer().get_text());
                                    let msg = cb_clone.emit(input_event);
                                    println!("got msg: {:?}", msg);
                                    program.dispatch(msg);
                                });
                            }
                            _ => {}
                        },
                        _ => (),
                    }
                }
                GtkWidget::TextInput(entry)
            }
            Widget::Block(txt) => textview(&txt),
            Widget::Checkbox(value) => {
                let cb = CheckButton::new_with_label("checkbox");
                if value {
                    cb.set_property("active", &value);
                }
                GtkWidget::Checkbox(cb)
            }
            Widget::Image(bytes) => {
                let image = Image::new();
                let pixbuf_loader =
                    PixbufLoader::new_with_mime_type("image/jpeg").expect("error loader");
                pixbuf_loader
                    .write(&bytes)
                    .expect("Unable to write svg data into pixbuf_loader");

                pixbuf_loader.close().expect("error creating pixbuf");

                let pixbuf = pixbuf_loader.get_pixbuf();

                image.set_from_pixbuf(Some(&pixbuf.expect("error in pixbuf_loader")));
                GtkWidget::Image(image)
            }
        }
    }
}

impl<APP, MSG> Backend<APP, MSG> for GtkBackend<APP, MSG>
where
    APP: Component<MSG> + 'static,
    MSG: Clone + Debug + 'static,
{
    fn init(app: APP) -> Rc<Self> {
        let mut rc_app = GtkBackend::new(app);
        rc_app.create_app();
        rc_app
    }
}

impl<APP, MSG> Dispatch<MSG> for GtkBackend<APP, MSG>
where
    MSG: Debug + 'static,
    APP: Component<MSG> + 'static,
{
    fn dispatch(self: &Rc<Self>, msg: MSG) {
        self.dispatch_inner(msg);
    }
}

enum GtkWidget {
    GBox(gtk::Box),
    Button(Button),
    Text(TextView),
    TextInput(Entry),
    Checkbox(CheckButton),
    Image(Image),
}
impl GtkWidget {
    fn as_container(&self) -> Option<&Container> {
        match self {
            GtkWidget::GBox(gbox) => {
                let container: &Container = gbox.upcast_ref();
                Some(container)
            }
            _ => None,
        }
    }

    fn as_widget(&self) -> Option<&gtk::Widget> {
        match self {
            GtkWidget::Button(btn) => {
                let widget: &gtk::Widget = btn.upcast_ref();
                Some(widget)
            }
            GtkWidget::GBox(cbox) => {
                let widget: &gtk::Widget = cbox.upcast_ref();
                Some(widget)
            }
            GtkWidget::Text(text_view) => {
                let widget: &gtk::Widget = text_view.upcast_ref();
                Some(widget)
            }
            GtkWidget::TextInput(textbox) => {
                let widget: &gtk::Widget = textbox.upcast_ref();
                Some(widget)
            }
            GtkWidget::Checkbox(checkbox) => {
                let widget: &gtk::Widget = checkbox.upcast_ref();
                Some(widget)
            }
            GtkWidget::Image(image) => {
                let widget: &gtk::Widget = image.upcast_ref();
                Some(widget)
            }
        }
    }

    fn add_children(&self, children: Vec<GtkWidget>) {
        if let Some(container) = self.as_container() {
            for child in children {
                if let Some(child_widget) = child.as_widget() {
                    container.add(child_widget);
                }
            }
        }
    }
}
impl From<Button> for GtkWidget {
    fn from(btn: Button) -> Self {
        GtkWidget::Button(btn)
    }
}

impl From<gtk::Box> for GtkWidget {
    fn from(gbox: gtk::Box) -> Self {
        GtkWidget::GBox(gbox)
    }
}

fn textview(txt: &str) -> GtkWidget {
    let buffer = TextBuffer::new(None::<&TextTagTable>);
    let text_view = TextView::new_with_buffer(&buffer);
    buffer.set_text(txt);
    GtkWidget::Text(text_view)
}
