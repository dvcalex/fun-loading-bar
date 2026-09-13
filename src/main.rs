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

    fn clean(&mut self, ch: char) {
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

#[derive(Clone)]
struct Keyframe {
    time_ms: u64,
    fb: Framebuffer,
}

fn max_frame_width(frames: Vec<Keyframe>) -> u16 {
    // get string blocks from frames
    let str_block_refs: Vec<&str> = frames.iter().map(|f| f.contents.as_str()).collect();

    // extract each line from blocks into flat vec
    let all_lines: Vec<&str> = str_block_refs.iter().flat_map(|s| s.lines()).collect();

    let mut max_width = 0;
    for line in all_lines {
        let line_len = line.chars().count() as u16;
        if line_len > max_width {
            max_width = line_len;
        }
    }
    max_width
}

fn max_frame_height(frames: Vec<Keyframe>) -> u16 {
    // get string blocks from frames
    let str_block_refs: Vec<&str> = frames.iter().map(|f| f.contents.as_str()).collect();

    let mut max_height: u16 = 0;
    for s in str_block_refs {
        let height = 1 + s.chars().filter(|&c| c == '\n').count() as u16;
        if height > max_height {
            max_height = height;
        }
    }
    max_height
}

// takes in raw contents and converts to vec of frame structs
fn make_frames(raw: &str) -> Result<Vec<Keyframe>, io::Error> {
    let mut frames = Vec::new();
    let mut cur_time_ms = 0;
    let mut cur_contents = String::new();

    for line in raw.lines() {
        if line.contains("---frame---") {
            // flush current content and build frame
            if !cur_contents.is_empty() {
                frames.push(Keyframe {
                    time_ms: cur_time_ms,
                    framebuffer: raw
                        .lines()
                        .map(|line| {
                            let mut row: Vec<char> = line.chars().collect();
                            row.resize(width as usize, ' ');
                            row
                        })
                        .collect(),
                });
                cur_contents.clear();
            }
        } else if line.starts_with("time_ms:") {
            // parse time for accumulating frame
            cur_time_ms = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        } else {
            // accumulate content lines
            if !line.is_empty() {
                cur_contents.push_str(line);
                cur_contents.push('\n');
            }
        }
    }

    // don't forget last frame!
    if !cur_contents.is_empty() {
        frames.push(Keyframe {
            time_ms: cur_time_ms,
            contents: cur_contents.trim_end().to_string(),
        });
    }

    Ok(frames)
}

fn fix_frame_sizes(frames: &mut Vec<Keyframe>, target_width: u16, target_height: u16) {
    for frame in frames {
        let mut rows: Vec<String> = frame
            .contents
            .lines()
            .map(|line| {
                let pad = target_width.saturating_sub(line.chars().count() as u16);
                format!("{}{}", line, " ".repeat(pad as usize))
            })
            .collect();

        rows.resize(target_height as usize, " ".repeat(target_width as usize));
        frame.contents = rows.join("\n");
    }
}

fn make_bar(total_len: u16, progress: f32) -> String {
    let filled = (total_len as f32 * progress) as usize;
    let empty = total_len as usize - filled;

    let bar = "█".repeat(filled) + &"░".repeat(empty);
    format!("\x1b[32m[{}]\x1b[0m ", bar)
}

fn draw<T: Write>(renderable: &str, writer: &mut T, width: u16, height: u16) {
    // make style for text
    let style = Style::new().italic().fg(Color::Green);

    // build grid of chars for frame
    let grid: Vec<Vec<char>> = renderable
        .lines()
        .map(|line| {
            let mut row: Vec<char> = line.chars().collect();
            row.resize(width as usize, ' ');
            row
        })
        .collect();

    draw_to(writer, DefRect::new(0, 0, width, height), |(x, y)| {
        let c = grid
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .copied()
            .unwrap_or(' ');
        AnsiChar(c, style)
    })
    .unwrap();
}

fn main() {
    let mut out = std::io::stdout();
    let contents = utils::read("res/frames.txt").unwrap();
    let mut frames = make_frames(&contents).unwrap();
    let frame_width = max_frame_width(frames.clone());
    let frame_height = max_frame_height(frames.clone());
    fix_frame_sizes(&mut frames, frame_width, frame_height);

    // initial clear and flush
    clear_all(&mut out).unwrap();
    out.flush().unwrap();

    let mut bar_ratio = 0.0; // test
    let bar_len = 10;
    let mut frame_idx = 0;

    while bar_ratio <= 1.0 {
        let frame = &frames[frame_idx % frames.len()];

        let renderable: String = make_bar(bar_len, bar_ratio);

        // clear last buffer and render new
        clear_area(&mut out, DefRect::new(0, 0, frame_width, frame_height)).unwrap();
        move_cursor_to(&mut out, (0, 0)).unwrap();
        draw(&frame.contents, &mut out, frame_width, frame_height);
        out.flush().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(frame.time_ms));

        bar_ratio += 0.01; // test
        frame_idx += 1;
    }

    // cleanup
    move_cursor_to(&mut out, (0, frame_height)).unwrap();
    out.flush().unwrap();
}
