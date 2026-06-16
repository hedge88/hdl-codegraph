class base_driver extends uvm_driver #(my_transaction);

    `uvm_component_utils(base_driver)

    // Properties
    protected int unsigned num_items;
    protected my_transaction req;

    // Constructor
    function new(string name = "base_driver", uvm_component parent = null);
        super.new(name, parent);
        num_items = 0;
    endfunction

    // Virtual methods
    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);
    endfunction

    virtual task run_phase(uvm_phase phase);
        forever begin
            seq_item_port.get_next_item(req);
            drive_item(req);
            seq_item_port.item_done();
        end
    endtask

    virtual task drive_item(my_transaction txn);
        num_items++;
    endtask

endclass
