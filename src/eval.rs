use crate::image::*;
use crate::image::blend::BlendMode;
use crate::parse::{ List, Spanned };
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum DataType {
    Nil,
    Number(f64),
    Str(String),
    Sym(String),
    Image(Image),
}

impl DataType {
    pub fn type_name(&self) -> &'static str {
        match self {
            DataType::Nil => "nil",
            DataType::Number(_) => "number",
            DataType::Str(_) => "string",
            DataType::Sym(_) => "symbol",
            DataType::Image(_) => "image",
        }
    }
}

#[derive(Debug)]
pub struct Env {
    vars: HashMap<String, DataType>,
    canvas: Option<Image>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            canvas: None,
        }
    }

    pub fn set(&mut self, name: String, value: DataType) {
        self.vars.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&DataType> {
        self.vars.get(name)
    }

    pub fn canvas(&self) -> Option<&Image> {
        self.canvas.as_ref()
    }

    pub fn canvas_mut(&mut self) -> Option<&mut Image> {
        self.canvas.as_mut()
    }
}

pub fn eval_expr<'a>(env: &mut Env, expr: Spanned<List>)
-> Result<DataType, chumsky::error::Rich<'a, String>> {
    let (expr, span) = expr;

    macro_rules! err {
        ($($arg:tt)*) => {
            Err(chumsky::error::Rich::custom(span, format!($($arg)*)))
        };
    }

    match expr {
        List::Error    => unimplemented!(),
        List::Nil      => unimplemented!(),

        List::Int(n)   => Ok(DataType::Number(n as f64)),
        List::Float(n) => Ok(DataType::Number(n)),
        List::Str(s)   => Ok(DataType::Str(s.to_string())),
        List::Sym(s)   => {
            if let Some(value) = env.get(s) {
                Ok(value.clone())
            } else {
                err!("undefined symbol: {}", s)
            }
        },

        List::Quote(x) => {
            match (*x).0 {
                List::Sym(s) => Ok(DataType::Sym(s.to_string())),
                _ => unimplemented!(),
            }
        },

        List::Cons(xs) if xs.is_empty() => err!("empty list cannot be evaluated"),
        List::Cons(xs) => {
            let mut iter = xs.into_iter();

            macro_rules! next_or {
                ($when_err:expr) => {
                    if let Some(item) = iter.next() {
                        item
                    } else {
                        return err!($when_err);
                    }
                };
            }

            macro_rules! next_or_default {
                ($default:expr) => {
                    if let Some(item) = iter.next() {
                        item
                    } else {
                        return Ok($default);
                    }
                };
            }

            let first = next_or!("missing function name");

            let (f, _) = first;

            macro_rules! check {
                ($name:ident, $type:ident, $expr:expr) => {{
                    let result = $expr;
                    if let DataType::$type($name) = result {
                        $name
                    } else {
                        return err!(
                            "{} must be of type {}, got {}",
                            stringify!($name),
                            stringify!($type),
                            result.type_name()
                        );
                    }
                }};
            }

            match f {
                List::Sym("canvas") => {
                    let width  = next_or!("missing width for `canvas`");
                    let height = next_or!("missing height for `canvas`");
                    let width  = eval_expr(env, width)?;
                    let height = eval_expr(env, height)?;

                    if let (DataType::Number(w), DataType::Number(h)) = (width, height) {
                        let img = Image::new(w as usize, h as usize);
                        env.canvas = Some(img);
                        Ok(DataType::Nil)
                    } else {
                        err!("width and height must be numbers")
                    }
                }

                List::Sym("img-load") => {
                    let path = next_or!("missing path for `load`");
                    let path = eval_expr(env, path)?;
                    if let DataType::Str(path) = path {
                        match Image::from_file(&path) {
                            Ok(img) => {
                                Ok(DataType::Image(img))
                            },
                            Err(e) => err!("failed to load image: {}", e),
                        }
                    } else {
                        err!("path must be a string")
                    }
                }

                List::Sym("img-render") => {
                    let image = next_or!("missing image for `render`");
                    let image = eval_expr(env, image)?;
                    if let DataType::Image(img) = image {
                        if let Some(canvas) = env.canvas_mut() {
                            canvas.blend_images(&img, BlendMode::Normal);
                            Ok(DataType::Nil)
                        } else {
                            err!("no canvas defined")
                        }
                    } else {
                        err!("image must be of type Image")
                    }
                }

                List::Sym("img-resize") => {
                    let image  = next_or!("missing image for `resize`");
                    let method = next_or!("missing resize method for `resize`");
                    let width  = next_or!("missing width for `resize`");
                    let height = next_or!("missing height for `resize`");
                    let image  = check!(image, Image, eval_expr(env, image)?);
                    let method = check!(method, Sym, eval_expr(env, method)?);
                    let width  = check!(width, Number, eval_expr(env, width)?);
                    let height = check!(height, Number, eval_expr(env, height)?);

                    let mut resized = image.clone();
                    match method.as_str() {
                        "nearest"
                        | "nearest-neighbor"
                        | "nearest-neighbour"
                        | "nn" => resized.resize_nearest_neighbour(width as usize, height as usize),
                        "bilinear"
                        | "b" => resized.resize_bilinear(width as usize, height as usize),
                        _ => return err!("unknown resize method: {}", method),
                    }
                    Ok(DataType::Image(resized))
                }

                List::Sym("img-move") => {
                    let image = next_or!("missing image for `move`");
                    let x     = next_or!("missing x offset for `move`");
                    let y     = next_or!("missing y offset for `move`");
                    let method = next_or_default!(DataType::Sym("pixels".to_string()));

                    let image = check!(image, Image, eval_expr(env, image)?);
                    let x     = check!(x, Number, eval_expr(env, x)?);
                    let y     = check!(y, Number, eval_expr(env, y)?);
                    let method = check!(method, Sym, eval_expr(env, method)?);

                    let mut new_image = image.clone();
                    // new_image.shift_with_empty(x as isize, y as isize);
                    match method.as_str() {
                        "pixel" | "pixels" | "px"
                            => new_image.shift_with_empty(x, y, false),
                        "frac" | "fract" | "fraction" | "fractions"
                            => new_image.shift_with_empty(x, y, true),
                        _ => return err!("unknown move method: {}", method),
                    }
                    Ok(DataType::Image(new_image))
                }

                List::Sym("img-mix") => {
                    // first = bottom, second = top
                    let image_a = next_or!("missing first image for `mix`");
                    let image_b = next_or!("missing second image for `mix`");
                    let mode    = next_or!("missing blend mode for `mix`");

                    let image_a = check!(image_a, Image, eval_expr(env, image_a)?);
                    let image_b = check!(image_b, Image, eval_expr(env, image_b)?);
                    let mode    = check!(mode, Sym, eval_expr(env, mode)?);

                    let blend_mode = match mode.as_str() {
                        "normal"   => BlendMode::Normal,
                        "multiply" => BlendMode::Multiply,
                        "screen"   => BlendMode::Screen,
                        "overlay"  => BlendMode::Overlay,
                        _ => return err!("unknown blend mode: {}", mode),
                    };

                    let mut new_image = image_a.clone();
                    new_image.blend_images(&image_b, blend_mode);
                    Ok(DataType::Image(new_image))
                }

                List::Sym("eff-blur") => {
                    let image  = next_or!("missing image for `blur`");
                    let radius = next_or!("missing radius for `blur`");

                    let image  = check!(image, Image, eval_expr(env, image)?);
                    let radius = check!(radius, Number, eval_expr(env, radius)?);

                    if radius < 0.0 {
                        return err!("blur radius cannot be negative");
                    }

                    let mut new_image = image.clone();
                    new_image.blur(radius as usize);
                    Ok(DataType::Image(new_image))
                }

                List::Sym("def") => {
                    let name  = next_or!("missing variable name for `def`");
                    let value = next_or!("missing value for variable");
                    let value = eval_expr(env, value)?;
                    env.set(name.0.to_string(), value);
                    Ok(DataType::Nil)
                },

                // (-> 1 (+ 2) ...) => (+ <1> 2)
                List::Sym("->") => {
                    let first = next_or!("missing first argument for `->`");
                    let fns = iter.collect::<Vec<_>>();

                    // if fns.is_empty() { return err!("empty `->`"); }
                    if fns.is_empty() {
                        return Ok(eval_expr(env, first)?);
                    }

                    let mut result_list = first;

                    // transform
                    for (f, s) in fns.into_iter() {
                        match f {
                            List::Sym(_) => {
                                // (-> 1 f) => (f <1>)
                                result_list = (List::Cons(vec![(f, s), result_list]), s);
                            },
                            List::Cons(xs) if xs.is_empty() => {
                                return err!("empty function in `->`");
                            },
                            List::Cons(xs) => {
                                // (-> 1 (f 2)) => (f <1> 2)
                                let mut xs = xs.into_iter();
                                let mut vec = vec![xs.next().unwrap(), result_list];
                                while let Some(item) = xs.next() {
                                    vec.push(item);
                                }
                                result_list = (List::Cons(vec), s);
                            },
                            e => {
                                return err!("{} is not supported in `->`", e);
                            }
                        }
                    }

                    Ok(eval_expr(env, result_list)?)
                },

                List::Sym("+")
                | List::Sym("-")
                | List::Sym("*")
                | List::Sym("/")
                | List::Sym("%") => {
                    let a = next_or!("missing first argument for arithmetic operation");
                    let b = next_or!("missing second argument for arithmetic operation");

                    let a = check!(a, Number, eval_expr(env, a)?);
                    let b = check!(b, Number, eval_expr(env, b)?);

                    let result = match f {
                        List::Sym("+") => a + b,
                        List::Sym("-") => a - b,
                        List::Sym("*") => a * b,
                        List::Sym("/") => {
                            if b == 0.0 {
                                return err!("division by zero");
                            }
                            a / b
                        },
                        List::Sym("%") => {
                            if b == 0.0 {
                                return err!("division by zero");
                            }
                            a % b
                        },
                        _ => unreachable!(),
                    };
                    Ok(DataType::Number(result))
                }

                _ => err!("unknown function: {}", f),
            }
        }
        List::Vec(_items) => unimplemented!(),
    }
}