
Handle structs are allowed beside the adapter: only the struct that implements `<Name>Port` must be named `<Name>Adapter`, and exactly one does; any other struct must implement a trait (the handle trait the port returns). A struct that implements nothing is still a finding (fixture `plain-struct`).
