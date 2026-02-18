use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::pixels::Color;

pub struct BitmapFont<'a> {
    texture: &'a Texture<'a>,
    base_x: i32,
    char_width: u32,
    char_height: u32,
}

impl<'a> BitmapFont<'a> {
    pub fn new(texture: &'a Texture<'a>) -> Self {
        Self {
            texture,
            base_x: 320,
            char_width: 10,
            char_height: 10,
        }
    }

    fn get_glyph_rect(&self, c: char) -> Rect {
        let mut x = 0;
        let mut y = 0;
        
        let c = c.to_ascii_lowercase();

        if c >= 'a' && c <= 'j' {
            x = (c as i32 - 'a' as i32) * 10;
            y = 0;
        } else if c >= 'k' && c <= 't' {
            x = (c as i32 - 'k' as i32) * 10 + 100; // 'k' start at 100? No, let's follow the pattern
            // Wait, previous file simplified: 
            // 'a'->0. 'j'->90.
            // If 'k'->100, then it continues on same row?
            // Let's re-read the code snippet carefully or assume linear.
            // sed output:
            // if (letter == 's') { x=180; y=10; } 
            // 's' is 19th char. 
            // if 'a' is 0,0. 's' at 180,10 means:
            // If it was one row: 's' would be 180. 
            // But y=10.
            // Let's assume standard 10 chars per row?
            // 'j' (9) -> 90.
            // 'k' (10) -> 0, 10? 
            // sed: 'z' -> 250, 10.
            // 's' (18) -> 180, 10.
            // Matches: x = (c - 'a') * 10. y = 0 if < 'k'? 
            // 's' index is 18. 18 * 10 = 180. y=10.
            // It seems x is global index * 10, but y shifts?
            // Let's stick to the explicit if-checks pattern for safety or a safe linear logic if confirmed.
            // "if (letter == 's') { TextSrcRect.x = 180; TextSrcRect.y = 10; }"
            // This is strange. x=180, y=10.
            // 's' is ASCII 115. 'a' is 97. 115-97 = 18. 18*10 = 180.
            // So x is indeed purely based on index (c - 'a') * 10.
            // AND y is set to 10.
            // This implies the texture has copies of the font? Or maybe the row logic I inferred earlier was wrong.
            // Let's trust the logic: x = (idx)*10. y depends on range?
            // Wait, if x=180 and y=10, and 'a' is x=0, y=0.
            // Then 'k' (10) should be x=100.
            // Let's check 'u' (20). x=200, y=10.
            // 'z' (25). x=250, y=10.
            // So A-J (0-9): y=0.
            // K-T (10-19): y=? Code for 's'(18) says y=10.
            // So K-Z seems to be y=10?
            // Let's use the exact matches from the cpp file logic where possible.
            
            x = (c as i32 - 'a' as i32) * 10;
            if c >= 'k' { y = 10; } // Based on 's' and 'z' having y=10.
            // What about 'a'...'j'? y=0.
        } else if c >= 'u' && c <= 'z' {
             x = (c as i32 - 'a' as i32) * 10;
             y = 10;
        }

        if c == ' ' { x = 0; y = 20; }
        else if c >= '0' && c <= '9' {
            // '0' -> x=10, y=20.
            // '1' -> x=20, y=20.
            // Index of '0' is 0. But x=10.
            x = (c as i32 - '0' as i32) * 10 + 10;
            y = 20;
        }
        
        // Offset
        x += self.base_x;
        
        Rect::new(x, y, self.char_width, self.char_height)
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>, x: i32, y: i32, text: &str, scale: f32) -> Result<(), String> {
        let mut current_x = x;
        
        for c in text.chars() {
            let src = self.get_glyph_rect(c);
            let dest_w = (self.char_width as f32 * scale) as u32;
            let dest_h = (self.char_height as f32 * scale) as u32;
            let dest = Rect::new(current_x, y, dest_w, dest_h);
            
            canvas.copy(self.texture, src, dest)?;
            
            current_x += dest_w as i32;
        }
        
        Ok(())
    }
}
