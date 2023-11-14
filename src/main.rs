use bmfont_rs::{Char, Chnl, Padding, Spacing};
use fontdue::layout::{Layout, CoordinateSystem, LayoutSettings, TextStyle};
use image::RgbaImage;
use texture_packer::{
    exporter::ImageExporter, TexturePacker, TexturePackerConfig, texture::Texture
};
use std::{fs::File, collections::BTreeMap};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Text to convert
    #[arg(short, long)]
    txt: Option<String>,

    /// txt path
    #[arg(long)]
    txt_path: Option<String>,

    /// export font size
    #[arg(long, default_value_t = 68)]
    font_size: u32,

    /// export font name
    #[arg(long)]
    font_name: String,

    /// export font path
    #[arg(short, long, default_value = ".")]
    path: String,

    /// ttf path
    #[arg(long)]
    ttf_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args { mut txt, txt_path, font_size, font_name, path, ttf_path } = Args::parse();
    let export_font_name: &str = &font_name;
    let export_path = std::path::Path::new(&path);
    let font_data = std::fs::read(&ttf_path)?;
    

    if txt.is_none() {
        if txt_path.is_some() {
            txt = Some(std::fs::read_to_string(&txt_path.unwrap())?);
        } else {
            panic!("txt or txt_path is none");
        }
    }

    println!("font size: {}", font_size);
    
    let txt:&str = &txt.unwrap();


    let settings = fontdue::FontSettings {
        scale: font_size as f32,
        ..fontdue::FontSettings::default()
    };
    let font = fontdue::Font::from_bytes(font_data, settings).unwrap();

    let mut font_map = BTreeMap::new();

    let config = TexturePackerConfig {
        max_width: 1024,
        max_height: 1024,
        allow_rotation: false,
        texture_outlines: false,
        border_padding: 2,
        ..Default::default()
    };

    let mut packer = TexturePacker::new_skyline(config);
    
    
    for ch in txt.chars() {
        let merrics = font.metrics(ch, font_size as f32);
        println!("ch: {}, m: {:?}", ch, merrics);
        let(metrics, bitmap) = font.rasterize(ch, font_size as f32);
        // println!("ch: {} w: {} h: {} len: {}", ch, metrics.width, metrics.height, bitmap.len());
        let mut image_buffer = RgbaImage::new(metrics.width as u32,  metrics.height as u32);
        image_buffer.fill(0);

        let mut i = 0;
        for y in 0 .. metrics.height {
            for x in 0 .. metrics.width {
                image_buffer.put_pixel(x as u32, y as u32, image::Rgba([255, 255, 255, bitmap[i]]));
                i = i + 1;
            }
        }
        
        if let Err(_) = packer.pack_own(ch, image_buffer) {
            panic!("texture pack fial");
        }
        font_map.insert(ch, metrics);
    }

    let exporter = ImageExporter::export(&packer).unwrap();
    
    let mut file = File::create( export_path.join(format!("{}.png", export_font_name))).unwrap();
    exporter
        .write_to(&mut file, image::ImageFormat::Png)
        .unwrap();

    println!("texture save in {:?}", file);

    let mut bm_font = bmfont_rs::Font::default();
    bm_font.info.face = format!("{}", export_font_name);
    bm_font.info.size = font_size as i16;
    bm_font.info.bold = false;
    bm_font.info.italic = false;
    bm_font.info.unicode = false;
    bm_font.info.stretch_h = 100;
    bm_font.info.smooth = true;
    bm_font.info.aa = 1;
    bm_font.info.padding = Padding::new(1, 1, 1, 1);
    bm_font.info.spacing = Spacing::new(2, 2);

    bm_font.common.line_height = (font_size as u16) + 10;
    bm_font.common.base = font_size as u16;
    bm_font.common.scale_w = packer.width() as u16;
    bm_font.common.scale_h = packer.height() as u16;
    bm_font.common.pages = 1;
    bm_font.common.packed = false;

    bm_font.pages.push(format!("{}.png", export_font_name));

    println!("{:?}", bm_font);
    let mut f_w = std::fs::File::create(export_path.join(format!("{}.fnt", export_font_name)))?;


    println!("texture size: {}x{}", packer.width(), packer.height());
    bm_font.chars.push(Char{
        id: 0,
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        xoffset: -1,
        yoffset: (font_size as i16) - 10,
        xadvance: font_size as i16,
        page: 0,
        chnl: Chnl::ALL,
    });

    bm_font.chars.push(Char{
        id: 32,
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        xoffset: -1,
        yoffset: (font_size as i16) - 10,
        xadvance: font_size as i16,
        page: 0,
        chnl: Chnl::ALL,
    });

    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    let fonts = &[font];

    for (name, frame) in packer.get_frames() {
        let metrics = font_map.get(name).unwrap();
        println!("    'id: {}'-'{}' : {:?} metrics: {:?}", name.clone() as u32, name, frame.frame.h, metrics);
        
        layout.reset(&LayoutSettings {
            ..LayoutSettings::default()
        });

        layout.append(fonts, &TextStyle::new(&format!("{}", name.clone()), font_size as f32, 0));

        println!("layout {:?}\n ==========", layout.glyphs());

        bm_font.chars.push(Char{
            id: name.clone().into(),
            x: frame.frame.x as u16,
            y: frame.frame.y as u16,
            width: frame.frame.w as u16,
            height: frame.frame.h as u16,
            xoffset: metrics.xmin as i16,
            yoffset: layout.glyphs()[0].y as i16,
            xadvance: font_size as i16,
            page: 0,
            chnl: Chnl::ALL,
        });
    }

    bm_font.chars.sort_by(|a, b| a.id.cmp(&b.id));

    bmfont_rs::text::to_writer(&mut f_w, &bm_font)?;
    println!("fnt save int {:?}", f_w);
    
    Ok(())
}