impl<'a> FFmpegCommand<'a> {
    /// Creates a new FFmpegCommand with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_file: None,
            output_file: None,
            overwrite: false,
            video_codec: None,
            audio_codec: None,
            format: None,
            video_filters: vec![],
            audio_filters: vec![],
            seek: None,
            duration: None,
            custom_options: vec![],
        }
    }

    /// Adds an input file to the command.
    #[must_use]
    pub fn with_input(mut self, input: &'a str) -> Self {
        self.input_file = Some(input);
        self
    }

    /// Adds an output file to the command.
    #[must_use]
    pub fn with_output(mut self, output: &'a str) -> Self {
        self.output_file = Some(output);
        self
    }

    /// Sets the overwrite flag.
    #[must_use]
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Sets the video codec.
    #[must_use]
    pub fn with_video_codec(mut self, codec: &'a str) -> Self {
        self.video_codec = Some(codec);
        self
    }

    /// Sets the audio codec.
    #[must_use]
    pub fn with_audio_codec(mut self, codec: &'a str) -> Self {
        self.audio_codec = Some(codec);
        self
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_format(mut self, format: &'a str) -> Self {
        self.format = Some(format);
        self
    }

    /// Adds a video filter.
    #[must_use]
    pub fn with_video_filter(mut self, filter: &'a str) -> Self {
        self.video_filters.push(filter);
        self
    }

    /// Adds an audio filter.
    #[must_use]
    pub fn with_audio_filter(mut self, filter: &'a str) -> Self {
        self.audio_filters.push(filter);
        self
    }

    /// Sets the seek position.
    #[must_use]
    pub fn with_seek(mut self, seek: &'a str) -> Self {
        self.seek = Some(seek);
        self
    }

    /// Sets the duration.
    #[must_use]
    pub fn with_duration(mut self, duration: &'a str) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Adds a custom option.
    #[must_use]
    pub fn with_custom_option(mut self, option: &'a str) -> Self {
        self.custom_options.push(option);
        self
    }

    /// Builds the FFmpeg command as a vector of arguments.
    #[must_use]
    pub fn build(&self) -> Vec<String> {
        // ... existing code ...
    }
}
