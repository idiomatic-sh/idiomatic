package main

import "testing"

func TestBannerUsesLocalPackage(t *testing.T) {
	if banner() != "todo api on :3000" {
		t.Fatal(banner())
	}
}
