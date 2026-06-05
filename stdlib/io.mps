// MPS Standard Library: io
// Input / Output utility functions

fn read_input(prompt: string) -> string {
    print(prompt)
    py_import readline
    let line: string = readline.readline("")
    return line.trim()
}
