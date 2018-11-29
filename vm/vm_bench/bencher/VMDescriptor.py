class VMDescriptor:
    def __init__(self, relative_vm_binary_path="", arg_str="", is_compiler_type=True):
        self.relative_vm_binary_path = relative_vm_binary_path
        self.arg_str = arg_str
        self.is_compiler_type = is_compiler_type
