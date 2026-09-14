mod utils;

use std::io;
use std::io::Write;
use terminal_draw::geometry::*;
use terminal_draw::*;

/*
Algorithm

- Read frames.txt, write to Vec<Keyframe>
- Preprocess Vec<Keyframe>
- for each time tick:
    - Write to framebuffer (Vec<Vec<char>>)
*/

#[derive(Clone)]
struct Framebuffer {
    buf_2d: Vec<Vec<char>>,
}

impl Framebuffer {
    fn get_width(&self) -> usize {
        self.buf_2d.first().map_or(0, |row| row.len())
    }

    fn get_height(&self) -> usize {
        self.buf_2d.len()
    }

    fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            buf_2d: vec![vec![' '; width]; height],
        }
    }

    fn clear(&mut self, ch: char) {
        for row in &mut self.buf_2d {
            for c in row.iter_mut() {
                *c = ch;
            }
        }
    }

    fn write(&mut self, buf_2d: &[Vec<char>], x: usize, y: usize) {
        let fb_height = self.buf_2d.len();
        let fb_width = self.buf_2d.first().map_or(0, |row| row.len());

        for (i, row) in buf_2d.iter().enumerate() {
            let fb_y = y + i;
            if fb_y >= fb_height {
                break; // past bottom row, nothing left
            }

            for (j, &ch) in row.iter().enumerate() {
                let fb_x = x + j;
                if fb_x >= fb_width {
                    break; // past the right edge, move on to next row
                }
                self.buf_2d[fb_y][fb_x] = ch;
            }
        }
    }

    fn out_to<W: Write>(&self, mut writer: W) {
        writer.write_all(b"data");
    }
}

fn make_bar(total_len: usize, progress: f32) -> String {
    let filled = (total_len as f32 * progress) as usize;
    let empty = total_len - filled;
    format!("[{}] ", "█".repeat(filled) + &"░".repeat(empty))
}

fn draw<T: Write>(fb: &Framebuffer, writer: &mut T, width: u16, height: u16) {
    // make style for text
    let style = Style::new().italic().fg(Color::Green);

    draw_to(writer, DefRect::new(0, 0, width, height), |(x, y)| {
        let c = fb
            .buf_2d
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .copied()
            .unwrap_or(' ');
        AnsiChar(c, style)
    })
    .unwrap();
}

fn get_keyframes(raw: &str) -> Vec<(String, u16)> {
    let mut frames = Vec::new();
    let mut cur_time_ms = 0;
    let mut cur_contents = String::new();

    for line in raw.lines() {
        if line.contains("---frame---") {
            if !cur_contents.is_empty() {
                frames.push((cur_contents.trim_end().to_string(), cur_time_ms));
                cur_contents.clear();
            }
        } else if line.starts_with("time_ms:") {
            cur_time_ms = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        } else if !line.is_empty() {
            cur_contents.push_str(line);
            cur_contents.push('\n');
        }
    }

    if !cur_contents.is_empty() {
        frames.push((cur_contents.trim_end().to_string(), cur_time_ms));
    }

    frames
}

fn str_to_char_buf_2d(s: &str) -> Vec<Vec<char>> {
    s.lines().map(|line| line.chars().collect()).collect()
}

// fn pad_char_buf_2d

fn main() {
    let mut out = std::io::stdout();
    let raw = utils::read("res/frames.txt").unwrap();

    // build vec of frames, where each frame is a tuple w/ 2d char buf and u16 time
    let frames: Vec<(Vec<Vec<char>>, u16)> = get_keyframes(&raw)
        .iter()
        .map(|(frame_contents, time_ms)| (str_to_char_buf_2d(frame_contents), *time_ms))
        .collect();

    // init values
    let width: u16 = 200;
    let height: u16 = 200;
    let mut bar_ratio = 0.0; // test
    let bar_len = 30;
    let mut frame_idx = 0;

    // build empty framebuffer
    let mut fb = Framebuffer::new(width as usize, height as usize);

    // first time clear
    clear_all(&mut out).unwrap();
    out.flush().unwrap();

    let pivot_x: usize = 20;
    let pivot_y: usize = 20;
    while bar_ratio <= 1.0 {
        let (my_art, time_ms) = &frames[frame_idx % frames.len()];

        fb.clear(' ');

        let bar = make_bar(bar_len, bar_ratio);
        let bar_buf = str_to_char_buf_2d(&bar);
        let bar_width = bar_buf.first().map_or(0, |r| r.len());
        for n in 0..3 {
            fb.write(&bar_buf, pivot_x + 1, pivot_y + 1 + n);
        }

        fb.write(my_art, pivot_x + bar_width + 1, pivot_y);

        // clear last buffer and render new
        clear_area(&mut out, DefRect::new(0, 0, width, height)).unwrap();
        move_cursor_to(&mut out, (0, 0)).unwrap();
        draw(&fb, &mut out, width, height);
        out.flush().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(*time_ms as u64));

        bar_ratio += 0.01; // test
        frame_idx += 1;
    }

    // cleanup
    move_cursor_to(&mut out, (0, height)).unwrap();
    out.flush().unwrap();
}
