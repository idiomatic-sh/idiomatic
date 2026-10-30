fn run(settings: &Settings, count: u8) { forward(settings, &mut count, make(), *settings); settings.go(count, 1); }
