class my_comp extends uvm_component;

    `uvm_component_utils(my_comp)

    function new(string name = "my_comp", uvm_component parent = null);
        super.new(name, parent);
    endfunction

endclass

class my_obj extends uvm_object;

    `uvm_object_utils(my_obj)

    function new(string name = "my_obj");
        super.new(name);
    endfunction

endclass
