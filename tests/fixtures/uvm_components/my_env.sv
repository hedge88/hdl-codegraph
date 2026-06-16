class my_env extends uvm_env;

    `uvm_component_utils(my_env)

    my_agent      agent;
    my_scoreboard scb;

    function new(string name = "my_env", uvm_component parent = null);
        super.new(name, parent);
    endfunction

    virtual function void build_phase(uvm_phase phase);
        super.build_phase(phase);

        agent = my_agent::type_id::create("agent", this);
        scb   = my_scoreboard::type_id::create("scb", this);
    endfunction

    virtual function void connect_phase(uvm_phase phase);
        super.connect_phase(phase);
        agent.mon.ap.connect(scb.analysis_export);
    endfunction

endclass
