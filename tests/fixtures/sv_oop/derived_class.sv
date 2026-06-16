class my_driver extends base_driver;

    `uvm_component_utils(my_driver)

    // Additional properties
    virtual my_if vif;
    int unsigned delay;

    function new(string name = "my_driver", uvm_component parent = null);
        super.new(name, parent);
        delay = 0;
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);
        if (!uvm_config_db#(virtual my_if)::get(this, "", "vif", vif))
            `uvm_fatal("NOVIF", "Virtual interface not set")
    endfunction

    virtual task drive_item(my_transaction txn);
        super.drive_item(txn);
        @(posedge vif.clk);
        vif.data <= txn.data;
        if (delay > 0) repeat(delay) @(posedge vif.clk);
    endtask

endclass
