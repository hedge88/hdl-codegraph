class my_sequence extends uvm_sequence #(my_transaction);

    `uvm_object_utils(my_sequence)

    function new(string name = "my_sequence");
        super.new(name);
    endfunction

    virtual task body();
        my_transaction txn;

        // uvm_do
        `uvm_do(req)

        // uvm_do_with
        `uvm_do_with(req, { write == 1'b1; })

        // uvm_create + uvm_send
        `uvm_create(req)
        req.addr = 32'hDEAD_BEEF;
        req.data = 64'hCAFE_BABE_0000_0001;
        req.write = 1'b1;
        `uvm_send(req)
    endtask

endclass

class write_sequence extends uvm_sequence #(my_transaction);

    `uvm_object_utils(write_sequence)

    int unsigned num_txns = 10;

    function new(string name = "write_sequence");
        super.new(name);
    endfunction

    virtual task body();
        repeat (num_txns) begin
            `uvm_do_with(req, { write == 1'b1; })
        end
    endtask

endclass
