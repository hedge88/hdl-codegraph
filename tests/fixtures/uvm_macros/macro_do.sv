class do_sequence extends uvm_sequence #(my_transaction);

    `uvm_object_utils(do_sequence)

    function new(string name = "do_sequence");
        super.new(name);
    endfunction

    virtual task body();
        my_transaction txn;

        // uvm_do: create, randomize, send
        `uvm_do(req)

        // uvm_do_with: create, randomize with constraint, send
        `uvm_do_with(req, { write == 1'b1; addr[1:0] == 0; })

        // uvm_create: create only (no randomize/send)
        `uvm_create(txn)
        txn.addr = 32'hBEEF;
        txn.data = 64'hCAFE;
        txn.write = 1'b0;

        // uvm_send: send without creating
        `uvm_send(txn)
    endtask

endclass
