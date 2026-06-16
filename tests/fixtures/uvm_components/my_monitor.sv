class my_monitor extends uvm_monitor;

    `uvm_component_utils(my_monitor)

    virtual my_if vif;
    uvm_analysis_port #(my_transaction) ap;

    function new(string name = "my_monitor", uvm_component parent = null);
        super.new(name, parent);
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);
        ap = new("ap", this);
        if (!uvm_config_db#(virtual my_if)::get(this, "", "vif", vif))
            `uvm_fatal("NOVIF", "Virtual interface not set for my_monitor")
    endfunction

    virtual task run_phase(uvm_phase phase);
        forever begin
            my_transaction txn;
            @(posedge vif.clk);
            if (vif.write) begin
                txn = my_transaction::type_id::create("txn");
                txn.addr  = vif.addr;
                txn.data  = vif.data;
                txn.write = vif.write;
                ap.write(txn);
            end
        end
    endtask

endclass
