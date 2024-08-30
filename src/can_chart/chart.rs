use slint::SharedPixelBuffer;

use crate::slint_generatedAppWindow::MouseData;
use plotters::*;
use prelude::*;

pub struct Chart {
    signal_name: String,
    y_offset_min: f32, // minimum value of y axis
    y_offset_max: f32, // maximum value of y axis
    x_offset_min: f32, // minimum value of x axis
    x_offset_max: f32, // maximum value of x axis
    range_x: i32,      // range value of x axis
    range_y: i32,      // range value of y axis
    width: u32,        // the width of the chart in pixels
    height: u32,       // the height of the chart in pixels
    current_draw_data: Vec<(f32, f32)>,
}

impl Chart {
    pub fn new(signal: String) -> Self {
        Self {
            signal_name: signal,
            y_offset_max: 100.0,
            y_offset_min: 0.0,
            x_offset_max: 100.0,
            x_offset_min: 0.0,
            range_x: 60,
            range_y: 60,
            width: 800,
            height: 600,
            current_draw_data: [(1.0, 1.0), (10.0, 10.0), (100.0, 100.0)].to_vec(),
        }
    }

    /// main function for rendering the chart with plotter
    pub fn render_plot(&mut self, mouse: MouseData) -> slint::Image {
        println!("Mouse data: {:?}", mouse);
        // clean all object on the chart
        if mouse.is_clean {
            self.current_draw_data.clear();
        }

        // Update width and height of chart when users resize the window
        if mouse.height != 0 && mouse.width != 0 {
            self.width = mouse.width as u32;
            self.height = mouse.height as u32;
        }

        // Init data for plotters
        let mut pixel_buffer = SharedPixelBuffer::new(self.width, self.height);
        let size = (pixel_buffer.width(), pixel_buffer.height());
        let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);
        let root = backend.into_drawing_area();
        // background: 181c27
        root.fill(&RGBColor(0x18, 0x1c, 0x27))
            .expect("error filling drawing area");

        // Init the first candle chart with x,y range
        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(self.range_x)
            .right_y_label_area_size(self.range_y)
            .caption(self.signal_name.clone(), ("Arial-Bold", 14).into_font())
            .build_cartesian_2d(
                self.x_offset_min..self.x_offset_max,
                self.y_offset_min..self.y_offset_max,
            )
            .expect("error building coordinate system");
        // Configure the x and y axes
        chart
            .configure_mesh()
            .disable_mesh()
            .axis_desc_style(("Arial", 12))
            .label_style(&WHITE)
            .axis_style(WHITE.stroke_width(1))
            .draw()
            .expect("error drawing axes");

        chart
            .draw_series(LineSeries::new(
                self.current_draw_data.clone(),
                RED.stroke_width(2),
            ))
            .unwrap()
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x, y)], RED));

        let _ = chart.plotting_area().draw(&Text::new(
            self.signal_name.clone(),
            (50.0, 50.0),
            ("Arial-Bold", 14).into_font().color(&WHITE),
        ));

        root.present().expect("error presenting");
        drop(chart);
        drop(root);

        slint::Image::from_rgb8(pixel_buffer)
    }
}
