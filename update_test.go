package main

import "testing"

func TestPicksLatestTag(t *testing.T) {
	input := "aaaaaaaa\trefs/tags/v1.2.9\nbbbbbbbb\trefs/tags/v1.10.0\ncccccccc\trefs/tags/v1.3.0"
	latest := pickLatestSemverTag(input)
	if latest != "v1.10.0" {
		t.Errorf("expected v1.10.0, got %s", latest)
	}
}

func TestIgnoresNonSemverTags(t *testing.T) {
	input := "aaaaaaaa\trefs/tags/latest\nbbbbbbbb\trefs/tags/v1.2\ncccccccc\trefs/tags/v2.0.0-rc.1\ndddddddd\trefs/tags/v2.0.0"
	latest := pickLatestSemverTag(input)
	if latest != "v2.0.0" {
		t.Errorf("expected v2.0.0, got %s", latest)
	}
}

func TestStripVPrefixFromSemverTag(t *testing.T) {
	if got := semverWithoutPrefix("v1.2.3"); got != "1.2.3" {
		t.Errorf("expected 1.2.3, got %s", got)
	}
	if got := semverWithoutPrefix("latest"); got != "" {
		t.Errorf("expected empty, got %s", got)
	}
}

func TestParsesSHAFromLsRemoteLine(t *testing.T) {
	line := "0123456789abcdef\trefs/tags/v1.0.0"
	sha := parseSHAFromLsRemoteLine(line)
	if sha != "0123456789abcdef" {
		t.Errorf("expected 0123456789abcdef, got %s", sha)
	}
}
