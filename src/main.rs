mod parse;
mod image;
mod eval;

use ariadne::{sources, Color, Label, Report, ReportKind};
use peak_alloc::PeakAlloc;

fn report(errs: Vec<chumsky::error::Rich<'_, String>>, path: String, src: &str) {
    errs.into_iter().for_each(|e| {
        Report::build(ReportKind::Error, (path.clone(), e.span().into_range()))
            .with_config(ariadne::Config::new()
                .with_compact(true)
                .with_index_type(ariadne::IndexType::Byte))
            .with_message(e.to_string())
            .with_label(
                Label::new((path.clone(), e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            // .with_labels(e.contexts().map(|(label, span)| {
            //     Label::new((path.clone(), span.into_range()))
            //         .with_message(format!("while parsing this {label}"))
            //         .with_color(Color::Yellow)
            // }))
            .finish()
            .print(sources([(path.clone(), src)]))
            .unwrap()
    });
}

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("no file specified");
            std::process::exit(1);
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let (tks, errs) = parse::lex(&src);

    let parse_errs = if let Some(tks) = &tks {
        let (lists, parse_errs) = parse::parse(tks, (src.len()..src.len()).into());

        if let Some(lists) = lists.filter(|_| errs.len() + parse_errs.len() == 0) {
            let mut env = eval::Env::new();
            let start_time = std::time::Instant::now();

            for e in lists {
                match eval::eval_expr(&mut env, e) {
                    Ok(_)  => (),
                    Err(e) => {
                        report(vec![e], path.clone(), &src);
                        return;
                    }
                }

            }

            let elapsed = start_time.elapsed();
            println!("Evaluation took: {:.2?}", elapsed);

            // write to file
            if let Some(canvas) = env.canvas() {
                if let Err(e) = canvas.to_file("output.png") {
                    eprintln!("Error saving image: {}", e);
                } else {
                    println!("Image saved to output.png");
                }
            } else {
                println!("No canvas defined, skipping image save.");
            }

            let current_mem = PEAK_ALLOC.current_usage_as_mb();
            println!("Used {} MB of RAM", current_mem);
            let peak_mem = PEAK_ALLOC.peak_usage_as_mb();
            println!("Peak {} MB", peak_mem);
        }

        parse_errs
    } else {
        vec![]
    };

    report(
        errs.into_iter()
            .map(|e| e.map_token(|c| c.to_string()))
            .chain(parse_errs.into_iter()
                .map(|e| e.map_token(|t| t.to_string())))
            .collect(),
        path, &src);
}
