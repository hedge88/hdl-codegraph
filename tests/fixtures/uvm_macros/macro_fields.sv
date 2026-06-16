class field_txn extends uvm_sequence_item;

    `uvm_object_utils(field_txn)

    rand bit [31:0] addr;
    rand bit [63:0] data;
    rand bit        write;
    string          label;

    function new(string name = "field_txn");
        super.new(name);
    endfunction

    virtual function void do_print(uvm_printer printer);
        `uvm_field_int(addr, UVM_ALL_ON)
        `uvm_field_int(data, UVM_ALL_ON)
        `uvm_field_int(write, UVM_ALL_ON)
        `uvm_field_string(label, UVM_ALL_ON)
    endfunction

endclass
