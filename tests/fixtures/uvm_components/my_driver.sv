class my_driver extends uvm_driver #(my_transaction);

    `uvm_component_utils(my_driver)

    virtual my_if vif;
    int unsigned drv_count;

    function new(string name = "my_driver", uvm_component parent = null);
        super.new(name, parent);
        drv_count = 0;
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);
        if (!uvm_config_db#(virtual my_if)::get(this, "", "vif", vif))
            `uvm_fatal("NOVIF", "Virtual interface not set for my_driver")
    endfunction

    virtual task run_phase(uvm_phase phase);
        forever begin
            seq_item_port.get_next_item(req);
            drive_item(req);
            seq_item_port.item_done();
            drv_count++;
        end
    endtask

    virtual task drive_item(my_transaction txn);
        @(posedge vif.clk);
        vif.addr  <= txn.addr;
        vif.data  <= txn.data;
        vif.write <= txn.write;
    endtask

endclass
