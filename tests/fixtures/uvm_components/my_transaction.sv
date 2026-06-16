class my_transaction extends uvm_sequence_item;

    `uvm_object_utils(my_transaction)

    rand bit [31:0] addr;
    rand bit [63:0] data;
    rand bit        write;

    constraint c_aligned {
        addr[1:0] == 2'b00;
    }

    function new(string name = "my_transaction");
        super.new(name);
    endfunction

    virtual function string convert2string();
        return $sformatf("addr=0x%0h data=0x%0h write=%0b", addr, data, write);
    endfunction

endclass
