class TestDescriptor:
    def __init__(self, test_folder_name="", test_generator_cmd="", test_generator_params={}):
        self.test_folder_name = test_folder_name
        self.test_generator_cmd = test_generator_cmd
        self.test_generator_params = test_generator_params
        self.test_full_path = ""
