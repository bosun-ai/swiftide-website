//! Snippets for `models/images`.

use swiftide::{
    chat_completion::{
        ChatCompletionRequest, ChatMessage, ChatMessageContentPart, ChatMessageContentSource,
    },
    integrations::openai::OpenAI,
    traits::ChatCompletion,
};

/// Sends a question and a PNG in one user message and prints the answer.
pub async fn image_request(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: image_request
    let bytes = std::fs::read("diagram.png")?;

    let message = ChatMessage::new_user_with_parts(vec![
        ChatMessageContentPart::text("Describe this diagram in one sentence."),
        ChatMessageContentPart::image(ChatMessageContentSource::bytes(bytes, Some("image/png"))),
    ]);

    let request = ChatCompletionRequest::builder()
        .messages(vec![message])
        .build()?;

    let response = openai.complete(&request).await?;
    println!("{}", response.message().unwrap_or_default());
    // ANCHOR_END: image_request
    Ok(())
}

/// The other ways to point an image part at its data.
pub fn image_sources() -> Vec<ChatMessageContentPart> {
    // ANCHOR: image_sources
    let by_url = ChatMessageContentPart::image("https://example.com/chart.png");
    let by_data_url = ChatMessageContentPart::image("data:image/png;base64,iVBORw0KGgo=");
    let by_file_id =
        ChatMessageContentPart::image(ChatMessageContentSource::file_id("file-abc123"));
    let in_s3 = ChatMessageContentPart::image(ChatMessageContentSource::s3(
        "s3://my-bucket/chart.png",
        None::<String>,
    ));
    let raw_bytes = ChatMessageContentPart::image_with_format(vec![0x89, 0x50, 0x4e, 0x47], "png");
    // ANCHOR_END: image_sources
    vec![by_url, by_data_url, by_file_id, in_s3, raw_bytes]
}

/// A PDF with a question about it, for providers that accept documents.
pub fn document_message(pdf: Vec<u8>) -> ChatMessage {
    // ANCHOR: document_message
    let message = ChatMessage::new_user_with_parts(vec![
        ChatMessageContentPart::text("List the action items in this report."),
        ChatMessageContentPart::document_with_name(
            ChatMessageContentSource::bytes(pdf, Some("application/pdf")),
            "report.pdf",
        ),
    ]);
    // ANCHOR_END: document_message
    message
}

/// A WAV recording. `ChatMessageContentPart::audio` leaves the format unset, so set it on the variant.
pub fn wav_audio(recording: Vec<u8>) -> ChatMessageContentPart {
    // ANCHOR: wav_audio
    let audio = ChatMessageContentPart::Audio {
        source: ChatMessageContentSource::bytes(recording, Some("audio/wav")),
        format: Some("wav".into()),
    };
    // ANCHOR_END: wav_audio
    audio
}
