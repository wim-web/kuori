package main

import "testing"

func newTask(name string) Task {
	return Task{
		Name:         name,
		Host:         "example.com",
		ScriptPath:   "script.sh",
		WorkingDir:   "/tmp",
		Sudo:         false,
		Environments: map[string]string{},
		TimeoutSec:   nil,
		Retry:        nil,
	}
}

func TestValidateFailsWhenTasksEmpty(t *testing.T) {
	config := Config{Tasks: []Task{}}
	if err := config.Validate(); err == nil {
		t.Error("expected error for empty tasks")
	}
}

func TestValidateFailsWhenNameIsBlank(t *testing.T) {
	task := newTask("deploy")
	task.Name = "   "
	config := Config{Tasks: []Task{task}}
	if err := config.Validate(); err == nil {
		t.Error("expected error for blank name")
	}
}

func TestValidateFailsWhenNamesDuplicated(t *testing.T) {
	config := Config{Tasks: []Task{newTask("deploy"), newTask("deploy")}}
	if err := config.Validate(); err == nil {
		t.Error("expected error for duplicated names")
	}
}

func TestValidateSucceedsWithValidTasks(t *testing.T) {
	config := Config{Tasks: []Task{newTask("deploy"), newTask("cleanup")}}
	if err := config.Validate(); err != nil {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestValidateFailsWhenTimeoutSecIsZero(t *testing.T) {
	task := newTask("deploy")
	zero := uint64(0)
	task.TimeoutSec = &zero
	config := Config{Tasks: []Task{task}}
	if err := config.Validate(); err == nil {
		t.Error("expected error for zero timeout_sec")
	}
}
