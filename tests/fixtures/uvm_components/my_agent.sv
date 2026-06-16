class my_agent extends uvm_agent;

    `uvm_component_utils(my_agent)

    my_driver    drv;
    my_monitor   mon;
    uvm_sequencer #(my_transaction) sqr;

    function new(string name = "my_agent", uvm_component parent = null);
        super.new(name, parent);
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);

        // Factory create
        drv = my_driver::type_id::create("drv", this);
        mon = my_monitor::type_id::create("mon", this);
        sqr = uvm_sequencer#(my_transaction)::type_id::create("sqr", this);

        // Config DB set
        uvm_config_db#(int)::set(this, "drv", "drv_count", 0);
    endfunction

    virtual function void connect_phase(uvm_phase phase);
        super.connect_phase(phase);
        drv.seq_item_port.connect(sqr.seq_item_export);
    endfunction

endclass
