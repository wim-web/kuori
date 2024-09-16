#Task: {
    name: string
    host: string
    script_path: string
    working_dir: string
    sudo: bool
    environments: [string]: string
}

#Config: {
    tasks: [...#Task]
}
