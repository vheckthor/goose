use lopdf::{content::Content as PdfContent, Document, Object};
use mcp_core::{Content, ToolError};
use std::{
    fs,
    path::{Path, PathBuf},
};

// Threshold for large text files (2MB - about half of the 4,194,304 bytes limit)
const LARGE_TEXT_THRESHOLD: usize = 2 * 1024 * 1024; // 2MB in bytes

pub async fn pdf_tool(
    path: &str,
    operation: &str,
    cache_dir: &Path,
) -> Result<Vec<Content>, ToolError> {
    // Open and parse the PDF file
    let doc = Document::load(path)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to open PDF file: {}", e)))?;

    let result = match operation {
        "extract_text" => {
            let mut text = String::new();

            // Iterate over each page in the document
            for (page_num, page_id) in doc.get_pages() {
                text.push_str(&format!("Page {}:\n", page_num));

                // Try to get text from page contents
                if let Ok(page_obj) = doc.get_object(page_id) {
                    if let Ok(page_dict) = page_obj.as_dict() {
                        // Try to get text from Contents stream
                        if let Ok(contents) =
                            page_dict.get(b"Contents").and_then(|c| c.as_reference())
                        {
                            if let Ok(content_obj) = doc.get_object(contents) {
                                if let Ok(stream) = content_obj.as_stream() {
                                    if let Ok(content_data) = stream.get_plain_content() {
                                        if let Ok(content) = PdfContent::decode(&content_data) {
                                            // Process each operation in the content stream
                                            for operation in content.operations {
                                                match operation.operator.as_ref() {
                                                    // "Tj" operator: show text
                                                    "Tj" => {
                                                        for operand in operation.operands {
                                                            if let Object::String(ref bytes, _) =
                                                                operand
                                                            {
                                                                if let Ok(s) =
                                                                    std::str::from_utf8(bytes)
                                                                {
                                                                    text.push_str(s);
                                                                }
                                                            }
                                                        }
                                                        text.push(' ');
                                                    }
                                                    // "TJ" operator: show text with positioning
                                                    "TJ" => {
                                                        if let Some(Object::Array(ref arr)) =
                                                            operation.operands.first()
                                                        {
                                                            let mut last_was_text = false;
                                                            for element in arr {
                                                                match element {
                                                                    Object::String(
                                                                        ref bytes,
                                                                        _,
                                                                    ) => {
                                                                        if let Ok(s) =
                                                                            std::str::from_utf8(
                                                                                bytes,
                                                                            )
                                                                        {
                                                                            if last_was_text {
                                                                                text.push(' ');
                                                                            }
                                                                            text.push_str(s);
                                                                            last_was_text = true;
                                                                        }
                                                                    }
                                                                    Object::Integer(offset) => {
                                                                        // Large negative offsets often indicate word spacing
                                                                        if *offset < -100 {
                                                                            text.push(' ');
                                                                            last_was_text = false;
                                                                        }
                                                                    }
                                                                    Object::Real(offset) => {
                                                                        if *offset < -100.0 {
                                                                            text.push(' ');
                                                                            last_was_text = false;
                                                                        }
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                            text.push(' ');
                                                        }
                                                    }
                                                    _ => (), // Ignore other operators
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                text.push('\n');
            }

            if text.trim().is_empty() {
                "No text found in PDF".to_string()
            } else {
                // Check if the extracted text is large
                let text_size = text.len();
                if text_size > LARGE_TEXT_THRESHOLD {
                    // Create a directory for large text files if it doesn't exist
                    let large_text_dir = cache_dir.join("large_pdf_texts");
                    fs::create_dir_all(&large_text_dir).map_err(|e| {
                        ToolError::ExecutionError(format!(
                            "Failed to create directory for large text: {}",
                            e
                        ))
                    })?;

                    // Create a filename based on the original PDF name
                    let pdf_path = PathBuf::from(path);
                    let pdf_filename = pdf_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unnamed_pdf");

                    let text_file_path = large_text_dir.join(format!("{}.txt", pdf_filename));

                    // Write the text to a file
                    fs::write(&text_file_path, &text).map_err(|e| {
                        ToolError::ExecutionError(format!(
                            "Failed to write large text to file: {}",
                            e
                        ))
                    })?;

                    // Format size in human-readable form
                    let size_str = if text_size < 1024 * 1024 {
                        format!("{:.2} KB", text_size as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", text_size as f64 / (1024.0 * 1024.0))
                    };

                    format!(
                        "Large text extracted from PDF ({})\n\n\
                        The extracted text is too large to display directly.\n\
                        Text has been written to: {}\n\n\
                        You can search through this file using ripgrep:\n\
                        rg 'search term' {}\n\n\
                        Or view portions of it:\n\
                        head -n 50 {}\n\
                        tail -n 50 {}\n\
                        less {}",
                        size_str,
                        text_file_path.display(),
                        text_file_path.display(),
                        text_file_path.display(),
                        text_file_path.display(),
                        text_file_path.display()
                    )
                } else {
                    format!("Extracted text from PDF:\n\n{}", text)
                }
            }
        }

        "extract_images" => {
            let cache_dir = cache_dir.join("pdf_images");
            fs::create_dir_all(&cache_dir).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create image cache directory: {}", e))
            })?;

            let mut images = Vec::new();
            let mut image_count = 0;

            // Helper function to determine file extension based on stream dict
            fn get_image_extension(dict: &lopdf::Dictionary) -> &'static str {
                if let Ok(filter) = dict.get(b"Filter") {
                    match filter {
                        Object::Name(name) => {
                            match name.as_slice() {
                                b"DCTDecode" => ".jpg",
                                b"JBIG2Decode" => ".jbig2",
                                b"JPXDecode" => ".jp2",
                                b"CCITTFaxDecode" => ".tiff",
                                b"FlateDecode" => {
                                    // PNG-like images often use FlateDecode
                                    // Check color space to confirm
                                    if let Ok(cs) = dict.get(b"ColorSpace") {
                                        if let Ok(name) = cs.as_name() {
                                            if name == b"DeviceRGB" || name == b"DeviceGray" {
                                                return ".png";
                                            }
                                        }
                                    }
                                    ".raw"
                                }
                                _ => ".raw",
                            }
                        }
                        Object::Array(filters) => {
                            // If multiple filters, check the last one
                            if let Some(Object::Name(name)) = filters.last() {
                                match name.as_slice() {
                                    b"DCTDecode" => return ".jpg",
                                    b"JPXDecode" => return ".jp2",
                                    _ => {}
                                }
                            }
                            ".raw"
                        }
                        _ => ".raw",
                    }
                } else {
                    ".raw"
                }
            }

            // Process each page
            for (page_num, page_id) in doc.get_pages() {
                let page = doc.get_object(page_id).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to get page {}: {}", page_num, e))
                })?;

                let page_dict = page.as_dict().map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to get page dict {}: {}",
                        page_num, e
                    ))
                })?;

                // Get page resources - handle both direct dict and reference
                let resources = match page_dict.get(b"Resources") {
                    Ok(res) => match res {
                        Object::Dictionary(dict) => Ok(dict),
                        Object::Reference(id) => doc
                            .get_object(*id)
                            .map_err(|e| {
                                ToolError::ExecutionError(format!(
                                    "Failed to get resource reference: {}",
                                    e
                                ))
                            })
                            .and_then(|obj| {
                                obj.as_dict().map_err(|e| {
                                    ToolError::ExecutionError(format!(
                                        "Resource reference is not a dictionary: {}",
                                        e
                                    ))
                                })
                            }),
                        _ => Err(ToolError::ExecutionError(
                            "Resources is neither dictionary nor reference".to_string(),
                        )),
                    },
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to get Resources: {}",
                        e
                    ))),
                }?;

                // Look for XObject dictionary - handle both direct dict and reference
                let xobjects = match resources.get(b"XObject") {
                    Ok(xobj) => match xobj {
                        Object::Dictionary(dict) => Ok(dict),
                        Object::Reference(id) => doc
                            .get_object(*id)
                            .map_err(|e| {
                                ToolError::ExecutionError(format!(
                                    "Failed to get XObject reference: {}",
                                    e
                                ))
                            })
                            .and_then(|obj| {
                                obj.as_dict().map_err(|e| {
                                    ToolError::ExecutionError(format!(
                                        "XObject reference is not a dictionary: {}",
                                        e
                                    ))
                                })
                            }),
                        _ => Err(ToolError::ExecutionError(
                            "XObject is neither dictionary nor reference".to_string(),
                        )),
                    },
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to get XObject: {}",
                        e
                    ))),
                };

                if let Ok(xobjects) = xobjects {
                    for (name, xobject) in xobjects.iter() {
                        let xobject_id = xobject.as_reference().map_err(|_| {
                            ToolError::ExecutionError("Failed to get XObject reference".to_string())
                        })?;

                        let xobject = doc.get_object(xobject_id).map_err(|e| {
                            ToolError::ExecutionError(format!("Failed to get XObject: {}", e))
                        })?;

                        if let Ok(stream) = xobject.as_stream() {
                            // Check if it's an image
                            if let Ok(subtype) =
                                stream.dict.get(b"Subtype").and_then(|s| s.as_name())
                            {
                                if subtype == b"Image" {
                                    let extension = get_image_extension(&stream.dict);

                                    // Get image metadata
                                    let width = stream
                                        .dict
                                        .get(b"Width")
                                        .and_then(|w| w.as_i64())
                                        .unwrap_or(0);
                                    let height = stream
                                        .dict
                                        .get(b"Height")
                                        .and_then(|h| h.as_i64())
                                        .unwrap_or(0);
                                    let bpc = stream
                                        .dict
                                        .get(b"BitsPerComponent")
                                        .and_then(|b| b.as_i64())
                                        .unwrap_or(0);

                                    // Get the image data
                                    if let Ok(data) = stream.get_plain_content() {
                                        let image_path = cache_dir.join(format!(
                                            "page{}_obj{}_{}{}",
                                            page_num,
                                            xobject_id.0,
                                            String::from_utf8_lossy(name),
                                            extension
                                        ));

                                        fs::write(&image_path, &data).map_err(|e| {
                                            ToolError::ExecutionError(format!(
                                                "Failed to write image: {}",
                                                e
                                            ))
                                        })?;

                                        images.push(format!(
                                            "Saved image to: {} ({}x{}, {} bits per component)",
                                            image_path.display(),
                                            width,
                                            height,
                                            bpc
                                        ));
                                        image_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if images.is_empty() {
                "No images found in PDF".to_string()
            } else {
                format!("Found {} images:\n{}", image_count, images.join("\n"))
            }
        }

        _ => {
            return Err(ToolError::InvalidParameters(format!(
                "Invalid operation: {}. Valid operations are: 'extract_text', 'extract_images'",
                operation
            )))
        }
    };

    Ok(vec![Content::text(result)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_pdf_text_extraction() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing text extraction from: {}", test_pdf_path.display());

        let result = pdf_tool(test_pdf_path.to_str().unwrap(), "extract_text", &cache_dir).await;

        assert!(result.is_ok(), "PDF text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();
        println!("Extracted text:\n{}", text);
        assert!(text.contains("Page 1"), "Should contain page marker");
        assert!(
            text.contains("This is a test PDF"),
            "Should contain expected test content"
        );
    }

    #[tokio::test]
    async fn test_pdf_image_extraction() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test_image.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing image extraction from: {}", test_pdf_path.display());

        // Now try image extraction
        let result = pdf_tool(
            test_pdf_path.to_str().unwrap(),
            "extract_images",
            &cache_dir,
        )
        .await;

        println!("Image extraction result: {:?}", result);
        assert!(result.is_ok(), "PDF image extraction should succeed");
        let content = result.unwrap();
        assert!(
            !content.is_empty(),
            "Image extraction result should not be empty"
        );
        let text = content[0].as_text().unwrap();
        println!("Extracted content: {}", text);

        // Should either find images or explicitly state none were found
        assert!(
            text.contains("Saved image to:") || text.contains("No images found"),
            "Should either save images or report none found"
        );

        // If we found images, verify they exist
        if text.contains("Saved image to:") {
            // Extract the file path from the output
            let file_path = text
                .lines()
                .find(|line| line.contains("Saved image to:"))
                .and_then(|line| line.split(": ").nth(1))
                .and_then(|path| path.split(" (").next())
                .expect("Should have a valid file path");

            println!("Verifying image file exists: {}", file_path);
            assert!(PathBuf::from(file_path).exists(), "Image file should exist");
        }
    }

    #[tokio::test]
    async fn test_pdf_invalid_path() {
        let cache_dir = tempfile::tempdir().unwrap().into_path();
        let result = pdf_tool("nonexistent.pdf", "extract_text", &cache_dir).await;

        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[tokio::test]
    async fn test_pdf_invalid_operation() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        let result = pdf_tool(
            test_pdf_path.to_str().unwrap(),
            "invalid_operation",
            &cache_dir,
        )
        .await;

        assert!(result.is_err(), "Should fail with invalid operation");
    }

    #[tokio::test]
    async fn test_large_pdf_text_extraction() {
        let large_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/visa-rules-public.pdf");

        // Skip test if the large PDF file doesn't exist (may not be committed to git)
        if !large_pdf_path.exists() {
            println!(
                "Skipping large PDF test as file doesn't exist: {}",
                large_pdf_path.display()
            );
            return;
        }

        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!(
            "Testing large text extraction from: {}",
            large_pdf_path.display()
        );

        let result = pdf_tool(large_pdf_path.to_str().unwrap(), "extract_text", &cache_dir).await;

        assert!(result.is_ok(), "Large PDF text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();

        // For large PDFs, we should get the message about writing to a file
        assert!(
            text.contains("Large text extracted from PDF"),
            "Should indicate large text was extracted"
        );
        assert!(
            text.contains("Text has been written to:"),
            "Should indicate where text was written"
        );

        // Extract the file path from the output and verify it exists
        let file_path = text
            .lines()
            .find(|line| line.contains("Text has been written to:"))
            .and_then(|line| line.split(": ").nth(1))
            .expect("Should have a valid file path");

        println!("Verifying text file exists: {}", file_path);
        assert!(PathBuf::from(file_path).exists(), "Text file should exist");

        // Verify file contains actual content
        let file_content = fs::read_to_string(file_path).expect("Should be able to read text file");
        assert!(!file_content.is_empty(), "Text file should not be empty");
        assert!(
            file_content.contains("Page 1"),
            "Should contain page marker"
        );
    }
}
