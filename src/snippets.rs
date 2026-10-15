use crate::{
    errors::AppError,
    models::{FontItem, FontSource},
};

pub fn css_snippet(item: &FontItem) -> String {
    let style = if item.is_italic { "italic" } else { "normal" };
    format!(
        "font-family: \"{}\", sans-serif;\nfont-weight: {};\nfont-style: {};",
        escape_css_string(&item.family),
        item.weight,
        style
    )
}

pub fn next_font_snippet(item: &FontItem) -> Result<String, AppError> {
    match item.source {
        FontSource::Google => {
            let family_id = item.source_family_id.as_deref().ok_or_else(|| {
                AppError::InvalidInput("Google font is missing its family ID.".into())
            })?;
            let import_name = google_import_identifier(family_id)?;
            let variable = google_variable_name(&import_name);
            let weight = json_string(&item.weight.to_string())?;
            let style = json_string(if item.is_italic { "italic" } else { "normal" })?;
            Ok(format!(
                "import {{ {import_name} }} from \"next/font/google\";\n\nconst {variable} = {import_name}({{\n  subsets: [\"latin\"],\n  weight: [{weight}],\n  style: [{style}],\n}});"
            ))
        }
        FontSource::Local
        | FontSource::Fontsource
        | FontSource::Fontshare
        | FontSource::NerdFonts => {
            let filename = item
                .file_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .filter(|name| !name.is_empty())
                .ok_or_else(|| AppError::InvalidInput("Local font has no filename.".into()))?;
            let filename = json_string(&format!("./{filename}"))?;
            let variable = lower_camel_case(&pascal_case_identifier(&item.family)?);
            Ok(format!(
                "import localFont from \"next/font/local\";\n\nconst {variable} = localFont({{\n  src: {filename},\n  display: \"swap\",\n}});"
            ))
        }
    }
}

fn escape_css_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\0' => escaped.push_str("\\fffd "),
            character if character.is_control() => {
                escaped.push('\\');
                escaped.push_str(&format!("{:x} ", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn json_string(value: &str) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|error| AppError::InvalidInput(error.to_string()))
}

fn google_import_identifier(value: &str) -> Result<String, AppError> {
    let words: Vec<String> = value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect();
    if words.is_empty() || !words[0].as_bytes()[0].is_ascii_alphabetic() {
        return Err(AppError::InvalidInput(format!(
            "Google font ID cannot form a Next.js import name: {value}"
        )));
    }
    Ok(words.join("_"))
}

fn google_variable_name(import_name: &str) -> String {
    let mut words = import_name.split('_');
    let first = words.next().unwrap_or_default().to_ascii_lowercase();
    first
        + &words
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<String>()
}

fn lower_camel_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn pascal_case_identifier(value: &str) -> Result<String, AppError> {
    let mut identifier = String::new();
    let mut capitalize_next = true;
    for character in value.chars() {
        if !character.is_ascii_alphanumeric() {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            identifier.push(character.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            identifier.push(character);
        }
    }
    if identifier.is_empty() || !identifier.as_bytes()[0].is_ascii_alphabetic() {
        return Err(AppError::InvalidInput(format!(
            "Font family ID cannot form a JavaScript identifier: {value}"
        )));
    }
    Ok(identifier)
}
